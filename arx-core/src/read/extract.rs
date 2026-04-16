use crate::codec::CodecId;
use crate::container::chunktab::{ChunkEntry, read_table};
use crate::container::manifest::Manifest;
use crate::container::superblock::{FLAG_ENCRYPTED, Superblock};
use crate::container::tail::{TAIL_LEN, TailSummary};
use crate::crypto::aead::{AeadKey, Region, derive_nonce};
use crate::error::Result;
use crate::util::sanitize::safe_join;

use blake3;
use std::fs::{self, File};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::Path;

#[derive(Clone, Default)]
pub struct ExtractOptions {
    pub aead_key: Option<[u8; 32]>,
    pub key_salt: [u8; 32],
    /// Derive the key from this password via Argon2id (uses the archive's stored kdf_salt).
    pub password: Option<String>,
}

pub fn extract(archive: &Path, dest: &Path, opts: Option<&ExtractOptions>) -> Result<()> {
    let mut f = File::open(archive)?;
    let sb = Superblock::read_from(&mut f)?;
    let header_len = sb.header_len();
    let enc_enabled = (sb.flags & FLAG_ENCRYPTED) != 0;

    let enc = resolve_enc(&sb, opts, enc_enabled)?;

    f.seek(SeekFrom::Start(header_len))?;
    let mut man_bytes = vec![0u8; sb.manifest_len as usize];
    f.read_exact(&mut man_bytes)?;

    let manifest_bytes = if let Some((ref key, salt)) = enc {
        let nonce = derive_nonce(&salt, Region::Manifest, 0);
        crate::crypto::aead::open_whole(key, &nonce, b"manifest", &man_bytes)?
    } else {
        man_bytes
    };

    let manifest: Manifest = ciborium::de::from_reader(&manifest_bytes[..])
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;

    f.seek(SeekFrom::Start(sb.chunk_table_off))?;
    let table_len = sb.data_off - sb.chunk_table_off;
    let mut table_bytes = vec![0u8; table_len as usize];
    f.read_exact(&mut table_bytes)?;

    let raw_table = if let Some((ref key, salt)) = enc {
        let nonce = derive_nonce(&salt, Region::ChunkTable, 0);
        crate::crypto::aead::open_whole(key, &nonce, b"chunktab", &table_bytes)?
    } else {
        table_bytes
    };

    let table = read_table(&mut &raw_table[..], sb.chunk_count)?;

    for d in &manifest.dirs {
        let p = safe_join(dest, &d.path)?;
        fs::create_dir_all(&p)?;
    }

    let mut buf = vec![0u8; 1 << 16];

    for fe in &manifest.files {
        eprintln!("extracting {}", fe.path);
        let outp = safe_join(dest, &fe.path)?;
        if let Some(parent) = outp.parent() {
            fs::create_dir_all(parent)?;
        }
        let mut out = File::create(&outp)?;

        for cref in &fe.chunk_refs {
            let ce: &ChunkEntry = &table[cref.id as usize];
            f.seek(SeekFrom::Start(ce.data_off))?;

            let mut cbuf = vec![0u8; ce.c_size as usize];
            f.read_exact(&mut cbuf)?;

            let comp = if let Some((ref key, salt)) = enc {
                let nonce = derive_nonce(&salt, Region::ChunkData, cref.id);
                crate::crypto::aead::open_whole(key, &nonce, b"chunk", &cbuf)?
            } else {
                cbuf
            };

            // Decompress
            let decompressed = decompress_chunk(&comp, ce.codec, ce.u_size, &mut buf)?;

            // Per-chunk blake3 integrity check (v4+ archives only — v3 entries have zero hash)
            if ce.blake3 != [0u8; 32] {
                let actual = *blake3::hash(&decompressed).as_bytes();
                if actual != ce.blake3 {
                    return Err(crate::error::ArxError::Format(format!(
                        "chunk {} blake3 mismatch: data corrupted",
                        cref.id
                    )));
                }
            }

            out.write_all(&decompressed)?;
        }

        if out.metadata()?.len() != fe.u_size {
            return Err(
                std::io::Error::new(std::io::ErrorKind::Other, "extracted size mismatch").into(),
            );
        }

        // Restore file permissions
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&outp, fs::Permissions::from_mode(fe.mode))?;
        }
    }

    // Restore symlinks (v4+ archives)
    #[cfg(unix)]
    for sl in &manifest.symlinks {
        let link_path = safe_join(dest, &sl.path)?;
        if let Some(parent) = link_path.parent() {
            fs::create_dir_all(parent)?;
        }
        // Remove existing file/link if present
        let _ = fs::remove_file(&link_path);
        std::os::unix::fs::symlink(&sl.target, &link_path)?;
    }

    Ok(())
}

pub fn verify(archive: &Path, opts: Option<&ExtractOptions>) -> Result<()> {
    let mut f = File::open(archive)?;
    let sb = Superblock::read_from(&mut f)?;
    let header_len = sb.header_len();
    let enc_enabled = (sb.flags & FLAG_ENCRYPTED) != 0;

    let tail = read_tail_at_eof(&mut f).map_err(|e| {
        std::io::Error::new(std::io::ErrorKind::Other, format!("tail read failed: {e}"))
    })?;

    let enc = resolve_enc(&sb, opts, enc_enabled)?;

    // 1) Manifest hash
    f.seek(SeekFrom::Start(header_len))?;
    let mut man_bytes = vec![0u8; sb.manifest_len as usize];
    f.read_exact(&mut man_bytes)?;
    let manifest_plain = if let Some((ref key, salt)) = enc {
        let nonce = derive_nonce(&salt, Region::Manifest, 0);
        crate::crypto::aead::open_whole(key, &nonce, b"manifest", &man_bytes)?
    } else {
        man_bytes
    };
    let got_manifest = *blake3::hash(&manifest_plain).as_bytes();

    // 2) ChunkTable hash
    f.seek(SeekFrom::Start(sb.chunk_table_off))?;
    let table_len = sb.data_off - sb.chunk_table_off;
    let mut table_bytes = vec![0u8; table_len as usize];
    f.read_exact(&mut table_bytes)?;
    let chunktab_plain = if let Some((ref key, salt)) = enc {
        let nonce = derive_nonce(&salt, Region::ChunkTable, 0);
        crate::crypto::aead::open_whole(key, &nonce, b"chunktab", &table_bytes)?
    } else {
        table_bytes
    };
    let got_tab = *blake3::hash(&chunktab_plain).as_bytes();

    let table = read_table(&mut &chunktab_plain[..], sb.chunk_count)?;

    let mut h_data = blake3::Hasher::new();
    let mut total_u = 0u64;
    let mut total_c = 0u64;

    for (id, ce) in table.iter().enumerate() {
        f.seek(SeekFrom::Start(ce.data_off))?;
        let mut cbuf = vec![0u8; ce.c_size as usize];
        f.read_exact(&mut cbuf)?;

        let comp_plain = if let Some((ref key, salt)) = enc {
            let nonce = derive_nonce(&salt, Region::ChunkData, id as u64);
            crate::crypto::aead::open_whole(key, &nonce, b"chunk", &cbuf)?
        } else {
            cbuf
        };

        h_data.update(&comp_plain);
        total_u = total_u.saturating_add(ce.u_size);
        total_c = total_c.saturating_add(comp_plain.len() as u64);
    }
    let got_data = *h_data.finalize().as_bytes();

    let ok = tail.manifest_blake3 == got_manifest
        && tail.chunktab_blake3 == got_tab
        && tail.data_blake3 == got_data
        && tail.total_u == total_u
        && tail.total_c == total_c;

    if !ok {
        return Err(
            std::io::Error::new(std::io::ErrorKind::Other, "verify mismatch (tail)").into(),
        );
    }

    Ok(())
}

// ── Helpers ──────────────────────────────────────────────────────────────────

/// Resolve encryption context from options + superblock.
fn resolve_enc(
    sb: &Superblock,
    opts: Option<&ExtractOptions>,
    enc_enabled: bool,
) -> Result<Option<(AeadKey, [u8; 32])>> {
    if !enc_enabled {
        return Ok(None);
    }
    let o = opts.ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::Other,
            "archive is encrypted; key required",
        )
    })?;

    // Raw key takes precedence over password
    if let Some(raw) = o.aead_key {
        return Ok(Some((AeadKey(raw), sb.kdf_salt)));
    }
    if let Some(pw) = &o.password {
        let key = crate::crypto::kdf::derive_key(pw, &sb.kdf_salt);
        return Ok(Some((AeadKey(key), sb.kdf_salt)));
    }
    Err(std::io::Error::new(
        std::io::ErrorKind::Other,
        "missing key or password for encrypted archive",
    )
    .into())
}

fn decompress_chunk(comp: &[u8], codec: u8, u_size: u64, buf: &mut Vec<u8>) -> Result<Vec<u8>> {
    match codec {
        x if x == CodecId::Store as u8 => Ok(comp.to_vec()),
        x if x == CodecId::Zstd as u8 => {
            let mut dec = zstd::stream::read::Decoder::with_buffer(comp)?;
            let mut out = Vec::with_capacity(u_size as usize);
            loop {
                let k = dec.read(buf)?;
                if k == 0 {
                    break;
                }
                out.extend_from_slice(&buf[..k]);
            }
            Ok(out)
        }
        _ => Err(std::io::Error::new(std::io::ErrorKind::Other, "unknown codec").into()),
    }
}

fn read_tail_at_eof(f: &mut File) -> std::io::Result<TailSummary> {
    let len = f.seek(SeekFrom::End(0))?;
    if len < TAIL_LEN {
        return Err(std::io::Error::new(
            std::io::ErrorKind::UnexpectedEof,
            "file too small for tail",
        ));
    }
    f.seek(SeekFrom::End(-(TAIL_LEN as i64)))?;
    TailSummary::read_from(f)
}
