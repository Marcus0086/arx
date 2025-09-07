use crate::codec::CodecId;
use crate::container::chunktab::{ChunkEntry, read_table};
use crate::container::manifest::Manifest;
use crate::container::superblock::{FLAG_ENCRYPTED, HEADER_LEN, Superblock};
use crate::container::tail::{TAIL_LEN, TailSummary};
use crate::crypto::aead::{AeadKey, Region, derive_nonce};
use crate::error::Result;

use blake3;
use std::fs::{self, File};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

#[derive(Clone, Default)]
pub struct ExtractOptions {
    pub aead_key: Option<[u8; 32]>,
    pub key_salt: [u8; 32],
}

pub fn extract(archive: &Path, dest: &Path, opts: Option<&ExtractOptions>) -> Result<()> {
    let mut f = File::open(archive)?;
    let sb = Superblock::read_from(&mut f)?;
    let enc_enabled = (sb.flags & FLAG_ENCRYPTED) != 0;

    let enc = if enc_enabled {
        let o = opts.ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::Other,
                "archive is encrypted; key required",
            )
        })?;
        let key = o
            .aead_key
            .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::Other, "missing aead_key"))?;
        Some((AeadKey(key), o.key_salt))
    } else {
        None
    };

    f.seek(SeekFrom::Start(HEADER_LEN))?;
    let mut man_bytes = vec![0u8; sb.manifest_len as usize];
    f.read_exact(&mut man_bytes)?;

    let manifest_bytes = if let Some((ref _key, _salt)) = enc {
        let nonce = derive_nonce(&_salt, Region::Manifest, 0);
        crate::crypto::aead::open_whole(&_key, &nonce, b"manifest", &man_bytes)
    } else {
        man_bytes
    };

    let manifest: Manifest = ciborium::de::from_reader(&manifest_bytes[..])
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;

    f.seek(SeekFrom::Start(sb.chunk_table_off))?;
    let table_len = sb.data_off - sb.chunk_table_off;
    let mut table_bytes = vec![0u8; table_len as usize];
    f.read_exact(&mut table_bytes)?;

    let raw_table = if let Some((ref _key, _salt)) = enc {
        let nonce = derive_nonce(&_salt, Region::ChunkTable, 0);
        crate::crypto::aead::open_whole(&_key, &nonce, b"chunktab", &table_bytes)
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
        let outp = safe_join(dest, &fe.path)?;
        if let Some(parent) = outp.parent() {
            fs::create_dir_all(parent)?;
        }
        let mut out = File::create(&outp)?;

        for cref in &fe.chunk_refs {
            let ce: &ChunkEntry = &table[cref.id as usize];
            f.seek(SeekFrom::Start(ce.data_off))?;

            // Read compressed (or stored) bytes for this chunk
            let mut cbuf = vec![0u8; ce.c_size as usize];
            f.read_exact(&mut cbuf)?;

            // Decrypt per-chunk if needed
            let comp = if let Some((ref _key, _salt)) = enc {
                let nonce = derive_nonce(&_salt, Region::ChunkData, cref.id);
                crate::crypto::aead::open_whole(&_key, &nonce, b"chunk", &cbuf)
            } else {
                cbuf
            };

            match ce.codec {
                x if x == CodecId::Store as u8 => {
                    out.write_all(&comp)?;
                }
                x if x == CodecId::Zstd as u8 => {
                    let mut dec = zstd::stream::read::Decoder::with_buffer(&comp[..])?;
                    loop {
                        let k = dec.read(&mut buf)?;
                        if k == 0 {
                            break;
                        }
                        out.write_all(&buf[..k])?;
                    }
                }
                _ => {
                    return Err(
                        std::io::Error::new(std::io::ErrorKind::Other, "unknown codec").into(),
                    );
                }
            }
        }

        if out.metadata()?.len() != fe.u_size {
            return Err(
                std::io::Error::new(std::io::ErrorKind::Other, "extracted size mismatch").into(),
            );
        }
    }

    Ok(())
}

fn safe_join(root: &Path, rel: &str) -> Result<PathBuf> {
    let p = Path::new(rel);
    if p.is_absolute() || rel.contains("../") || rel.contains("..\\") {
        return Err(
            std::io::Error::new(std::io::ErrorKind::Other, format!("unsafe path: {rel}")).into(),
        );
    }
    Ok(root.join(p))
}

pub fn verify(archive: &Path, opts: Option<&ExtractOptions>) -> Result<()> {
    let mut f = File::open(archive)?;
    let sb = Superblock::read_from(&mut f)?;
    let enc_enabled = (sb.flags & FLAG_ENCRYPTED) != 0;

    // Locate and read tail
    let tail = read_tail_at_eof(&mut f).map_err(|e| {
        std::io::Error::new(std::io::ErrorKind::Other, format!("tail read failed: {e}"))
    })?;

    // AEAD if needed
    let enc = if enc_enabled {
        let o = opts.ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::Other,
                "archive is encrypted; key required",
            )
        })?;
        let key = o
            .aead_key
            .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::Other, "missing aead_key"))?;
        Some((AeadKey(key), o.key_salt))
    } else {
        None
    };

    // 1) Manifest hash (plaintext)
    f.seek(SeekFrom::Start(HEADER_LEN))?;
    let mut man_bytes = vec![0u8; sb.manifest_len as usize];
    f.read_exact(&mut man_bytes)?;
    let manifest_plain = if let Some((ref _key, ref _salt)) = enc {
        let nonce = derive_nonce(_salt, Region::Manifest, 0);
        crate::crypto::aead::open_whole(_key, &nonce, b"manifest", &man_bytes)
    } else {
        man_bytes
    };
    let mut h_manifest = blake3::Hasher::new();
    h_manifest.update(&manifest_plain);
    let got_manifest = h_manifest.finalize();

    // 2) ChunkTable hash (plaintext)
    f.seek(SeekFrom::Start(sb.chunk_table_off))?;
    let table_len = sb.data_off - sb.chunk_table_off;
    let mut table_bytes = vec![0u8; table_len as usize];
    f.read_exact(&mut table_bytes)?;
    let chunktab_plain = if let Some((ref _key, ref _salt)) = enc {
        let nonce = derive_nonce(_salt, Region::ChunkTable, 0);
        crate::crypto::aead::open_whole(_key, &nonce, b"chunktab", &table_bytes)
    } else {
        table_bytes
    };
    let mut h_tab = blake3::Hasher::new();
    h_tab.update(&chunktab_plain);
    let got_tab = h_tab.finalize();

    let table = read_table(&mut &chunktab_plain[..], sb.chunk_count)?;

    let mut h_data = blake3::Hasher::new();
    let mut total_u = 0u64;
    let mut total_c = 0u64;

    for (id, ce) in table.iter().enumerate() {
        f.seek(SeekFrom::Start(ce.data_off))?;
        let mut cbuf = vec![0u8; ce.c_size as usize];
        f.read_exact(&mut cbuf)?;

        let comp_plain = if let Some((ref _key, ref _salt)) = enc {
            let nonce = derive_nonce(_salt, Region::ChunkData, id as u64);
            crate::crypto::aead::open_whole(_key, &nonce, b"chunk", &cbuf)
        } else {
            cbuf
        };

        h_data.update(&comp_plain);
        total_u = total_u.saturating_add(ce.u_size);
        total_c = total_c.saturating_add(comp_plain.len() as u64);
    }
    let got_data = h_data.finalize();

    // Compare with Tail
    let ok = tail.manifest_blake3 == *got_manifest.as_bytes()
        && tail.chunktab_blake3 == *got_tab.as_bytes()
        && tail.data_blake3 == *got_data.as_bytes()
        && tail.total_u == total_u
        && tail.total_c == total_c;

    if !ok {
        return Err(
            std::io::Error::new(std::io::ErrorKind::Other, "verify mismatch (tail)").into(),
        );
    }

    Ok(())
}

fn read_tail_at_eof(f: &mut File) -> std::io::Result<TailSummary> {
    use crate::container::tail::TailSummary as TS; // to access read_from if impl’d
    let len = f.seek(SeekFrom::End(0))?;
    if len < TAIL_LEN {
        return Err(std::io::Error::new(
            std::io::ErrorKind::UnexpectedEof,
            "file too small for tail",
        ));
    }
    f.seek(SeekFrom::End(-(TAIL_LEN as i64)))?;

    TS::read_from(f)
}
