use crate::container::chunktab::{ENTRY_SIZE, read_table_from_slice};
use crate::container::manifest::Manifest;
use crate::container::superblock::{FLAG_ENCRYPTED, HEADER_LEN, Superblock};
use crate::container::tail::{TAIL_LEN, TAIL_MAGIC};
use crate::crypto::aead::{AeadKey, Region, derive_nonce};
use crate::error::Result;

use std::env;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;

#[derive(Clone, Default)]
pub struct ListOptions {
    pub aead_key: Option<[u8; 32]>,
    pub key_salt: [u8; 32],
}

pub fn list(archive: &Path, opts: Option<&ListOptions>) -> Result<()> {
    let mut f = File::open(archive)?;
    let file_len = f.metadata()?.len();
    let dbg = env::var_os("ARX_DEBUG_LIST").is_some();

    let sb = Superblock::read_from(&mut f)?;
    let enc_enabled = (sb.flags & FLAG_ENCRYPTED) != 0;

    if dbg {
        eprintln!(
            "[DBG] SB: ver={} flags=0x{:x}\n      manifest_len={}  HEADER_LEN={}  manifest_end={}\n      chunk_table_off={}  data_off={}  chunk_count={}\n      file_len={}",
            sb.version,
            sb.flags,
            sb.manifest_len,
            HEADER_LEN,
            HEADER_LEN + sb.manifest_len,
            sb.chunk_table_off,
            sb.data_off,
            sb.chunk_count,
            file_len
        );
    }

    let mut file_end_for_data = file_len;
    if file_len >= TAIL_LEN {
        f.seek(SeekFrom::End(-(TAIL_LEN as i64)))?;
        let mut magic = [0u8; 8];
        if f.read_exact(&mut magic).is_ok() && magic == TAIL_MAGIC {
            file_end_for_data = file_len - TAIL_LEN;
            if dbg {
                eprintln!(
                    "[DBG] Tail detected at off={} (TAIL_LEN={})",
                    file_end_for_data, TAIL_LEN
                );
            }
        } else if dbg {
            eprintln!("[DBG] No tail magic at EOF (optional in alpha)");
        }
    }

    let manifest_end = HEADER_LEN.checked_add(sb.manifest_len).ok_or_else(|| {
        std::io::Error::new(std::io::ErrorKind::InvalidData, "manifest_len overflow")
    })?;
    if manifest_end > file_end_for_data {
        return Err(std::io::Error::new(
            std::io::ErrorKind::UnexpectedEof,
            format!(
                "manifest end {} > file end for data {}",
                manifest_end, file_end_for_data
            ),
        )
        .into());
    }
    if sb.chunk_table_off < HEADER_LEN || sb.chunk_table_off > file_end_for_data {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!(
                "bad chunk_table_off {} (file_end_for_data {})",
                sb.chunk_table_off, file_end_for_data
            ),
        )
        .into());
    }
    if sb.data_off < sb.chunk_table_off || sb.data_off > file_end_for_data {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!(
                "bad data_off {} (table_off {}, file_end_for_data {})",
                sb.data_off, sb.chunk_table_off, file_end_for_data
            ),
        )
        .into());
    }
    let table_ct_len = sb.data_off - sb.chunk_table_off;
    if dbg {
        eprintln!(
            "[DBG] Derived: table_len={} (= data_off - chunk_table_off)",
            table_ct_len
        );
    }

    let enc = if enc_enabled {
        let o = opts.ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::Other,
                "archive is encrypted; --key/--key-salt required",
            )
        })?;
        let key = o.aead_key.ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::Other,
                "missing --key for encrypted archive",
            )
        })?;
        if dbg {
            eprintln!(
                "[DBG] AEAD: enabled. key=32B provided, salt={:02x?}..",
                &o.key_salt[..4]
            );
        }
        Some((AeadKey(key), o.key_salt))
    } else {
        if dbg {
            eprintln!("[DBG] AEAD: disabled");
        }
        None
    };

    f.seek(SeekFrom::Start(HEADER_LEN))?;
    if dbg {
        eprintln!(
            "[DBG] Reading manifest: off={} len={}",
            HEADER_LEN, sb.manifest_len
        );
    }
    let mut mbytes = vec![0u8; sb.manifest_len as usize];
    f.read_exact(&mut mbytes)?;
    if dbg {
        eprintln!("[DBG] Manifest bytes: ct_len={}", mbytes.len());
    }

    let manifest_bytes = if let Some((ref key, salt)) = enc {
        let nonce = derive_nonce(&salt, Region::Manifest, 0);
        let pt = crate::crypto::aead::open_whole(key, &nonce, b"manifest", &mbytes);
        if dbg {
            eprintln!("[DBG] Manifest decrypted: pt_len={}", pt.len());
        }
        pt
    } else {
        mbytes
    };

    let manifest: Manifest = match ciborium::de::from_reader(&manifest_bytes[..]) {
        Ok(m) => m,
        Err(e) => {
            if dbg {
                eprintln!(
                    "[DBG] Manifest CBOR decode error: {} (pt_len={})",
                    e,
                    manifest_bytes.len()
                );
            }
            return Err(std::io::Error::new(std::io::ErrorKind::Other, e).into());
        }
    };
    if dbg {
        eprintln!(
            "[DBG] Manifest parsed: files={} dirs={}",
            manifest.files.len(),
            manifest.dirs.len()
        );
    }

    f.seek(SeekFrom::Start(sb.chunk_table_off))?;
    if dbg {
        eprintln!(
            "[DBG] Reading chunk table: off={} len={} (ciphertext)",
            sb.chunk_table_off, table_ct_len
        );
    }
    let mut tbytes = vec![0u8; table_ct_len as usize];
    f.read_exact(&mut tbytes)?;
    if dbg {
        eprintln!(
            "[DBG] Chunk table bytes: ct_len={} data_off={} chunk_table_off={} table_len={}",
            tbytes.len(),
            sb.data_off,
            sb.chunk_table_off,
            table_ct_len
        );
    }

    let raw_table = if let Some((ref key, salt)) = enc {
        let nonce = derive_nonce(&salt, Region::ChunkTable, 0);
        let pt = crate::crypto::aead::open_whole(key, &nonce, b"chunktab", &tbytes);
        if dbg {
            eprintln!("[DBG] Chunk table decrypted: pt_len={}", pt.len());
        }
        pt
    } else {
        tbytes
    };

    let expected_pt_len = sb.chunk_count as usize * ENTRY_SIZE;
    if raw_table.len() != expected_pt_len {
        return Err(std::io::Error::new(
            std::io::ErrorKind::UnexpectedEof,
            format!(
                "chunk table size mismatch: got {} bytes (plaintext), expected {} ({} entries * {})",
                raw_table.len(), expected_pt_len, sb.chunk_count, ENTRY_SIZE
            ),
        ).into());
    }

    let table = match read_table_from_slice(&mut &raw_table[..], sb.chunk_count) {
        Ok(t) => t,
        Err(e) => {
            if dbg {
                eprintln!(
                    "[DBG] read_table error: {} (pt_len={}, expected entries={})",
                    e,
                    raw_table.len(),
                    sb.chunk_count
                );
            }
            return Err(e.into());
        }
    };

    if dbg {
        eprintln!("[DBG] Chunk table parsed: entries={}", table.len());
        for (i, ce) in table.iter().enumerate() {
            let end = ce.data_off.saturating_add(ce.c_size);
            // If tail exists, chunks must be within [data_off, file_end_for_data]
            let bad = ce.data_off < sb.data_off || end > file_end_for_data;
            eprintln!(
                "[DBG]  CE[{}]: codec={} u={} c={} off={} end={} {}",
                i,
                ce.codec,
                ce.u_size,
                ce.c_size,
                ce.data_off,
                end,
                if bad { "<-- OUT OF BOUNDS!" } else { "" }
            );
        }
    }

    // Hard bound check (fail fast if any entry overlaps tail or precedes data_off)
    for (i, ce) in table.iter().enumerate() {
        if ce.data_off < sb.data_off {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!(
                    "chunk[{}] data_off {} < data_off {}",
                    i, ce.data_off, sb.data_off
                ),
            )
            .into());
        }
        let end = ce.data_off.saturating_add(ce.c_size);
        if end > file_end_for_data {
            return Err(std::io::Error::new(
                std::io::ErrorKind::UnexpectedEof,
                format!(
                    "chunk[{}] end {} exceeds file_end_for_data {}",
                    i, end, file_end_for_data
                ),
            )
            .into());
        }
    }

    let enc_mark = if enc_enabled { " [E]" } else { "" };
    for fe in &manifest.files {
        let mut c_sum = 0u64;
        for c in &fe.chunk_refs {
            c_sum += table[c.id as usize].c_size;
        }
        println!(
            "{}{}  u={}  c={}  chunks={}",
            fe.path,
            enc_mark,
            fe.u_size,
            c_sum,
            fe.chunk_refs.len()
        );
    }

    Ok(())
}
