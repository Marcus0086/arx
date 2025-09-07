use crate::chunking::fastcdc::{ChunkParams, StreamingChunker};
use crate::codec::zstdc::ZstdCompressor;
use crate::codec::{CodecId, Compressor};
use crate::container::chunktab::{ChunkEntry, ENTRY_SIZE, write_table};
use crate::container::manifest::{ChunkRef, DirEntry, FileEntry, Manifest, Meta};
use crate::container::superblock::{FLAG_ENCRYPTED, HEADER_LEN, Superblock, VERSION};
use crate::container::tail::TailSummary;
use crate::crypto::aead::{AeadKey, Region, TAG_LEN, derive_nonce, seal_whole};
use crate::error::Result;

use blake3;
use rayon::prelude::*;
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use time::OffsetDateTime;
use walkdir::WalkDir;

#[derive(Clone, Default)]
pub struct PackOptions {
    /// When true, zero timestamps in manifest for deterministic output.
    pub deterministic: bool,
    /// Only accept compression if it saves at least this fraction (e.g. 0.05 = 5%).
    pub min_gain: f32, // default 0.05 if left as 0.0
    /// Optional raw 32-byte key for AEAD (alpha).
    pub aead_key: Option<[u8; 32]>,
    /// Salt for nonce derivation; for deterministic builds, pass all-zero.
    pub key_salt: [u8; 32],
}

struct CountingWriter<'a, W: Write> {
    inner: &'a mut W,
    n: u64,
}
impl<'a, W: Write> CountingWriter<'a, W> {
    fn new(inner: &'a mut W) -> Self {
        Self { inner, n: 0 }
    }
    #[allow(dead_code)]
    fn bytes(&self) -> u64 {
        self.n
    }
}
impl<'a, W: Write> Write for CountingWriter<'a, W> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let k = self.inner.write(buf)?;
        self.n += k as u64;
        Ok(k)
    }
    fn flush(&mut self) -> std::io::Result<()> {
        self.inner.flush()
    }
}

fn mode_from(md: &std::fs::Metadata) -> u32 {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        md.permissions().mode()
    }
    #[cfg(not(unix))]
    {
        0o100644
    }
}
fn mtime_from(md: &std::fs::Metadata) -> i64 {
    md.modified()
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}
fn rel_display(path: &Path, roots: &[&Path]) -> Result<String> {
    for r in roots {
        if let Ok(p) = path.strip_prefix(r) {
            return Ok(p.to_string_lossy().to_string());
        }
    }
    Ok(path.to_string_lossy().to_string())
}

fn effective_min_gain(opts: Option<&PackOptions>) -> f32 {
    let val = opts.map(|o| o.min_gain).unwrap_or(0.05);
    if val <= 0.0 { 0.05 } else { val }
}
fn should_compress(u: usize, c: usize, min_gain: f32) -> bool {
    (u as f64 - c as f64) >= (u as f64 * min_gain as f64)
}

// Planning structs
#[derive(Clone)]
struct NewChunk {
    hash: [u8; 32],
    u_size: u64,
    c_size: u64, // compressed size (without AEAD tag)
    codec: u8,
    file_off: u64, // offset into the source file where this chunk starts
}
#[derive(Clone)]
struct FilePlan {
    path: PathBuf,
    mode: u32,
    mtime: i64,
    u_size: u64,
    chunks: Vec<NewChunk>,
}
struct ChunkPlan {
    src: PathBuf,
    off: u64,
    len: u64,
    codec: u8,
}

pub fn pack(inputs: &[&Path], out: &Path, opts: Option<&PackOptions>) -> Result<()> {
    // ── Walk inputs ──────────────────────────────────────────────────────────
    let mut files: Vec<PathBuf> = Vec::new();
    let mut dirs: Vec<PathBuf> = Vec::new();
    for root in inputs {
        for e in WalkDir::new(root).follow_links(false) {
            let e = e.map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
            let p = e.path();
            if e.file_type().is_dir() {
                dirs.push(p.to_path_buf());
            } else if e.file_type().is_file() {
                files.push(p.to_path_buf());
            }
        }
    }
    dirs.sort();
    files.sort();

    // ── Plan chunks per file (parallel) ──────────────────────────────────────
    let min_gain = effective_min_gain(opts);
    let params = ChunkParams::default();
    let zstd = ZstdCompressor;

    let file_plans: Vec<FilePlan> = files
        .par_iter()
        .map(|src_path| -> Result<FilePlan> {
            let meta = fs::metadata(src_path)?;
            let mut f = File::open(src_path)?;
            let mut chunker = StreamingChunker::new(params);
            let mut buf = Vec::<u8>::with_capacity(params.avg);
            let mut chunks = Vec::<NewChunk>::new();
            let mut total_u = 0u64;
            let mut file_off = 0u64;

            loop {
                let n = chunker.next_chunk(&mut f, &mut buf)?;
                if n == 0 {
                    break;
                }
                total_u += n as u64;

                // Hash (uncompressed)
                let hash = blake3::hash(&buf[..n]);

                // Trial compress to measure c_size
                let mut tmp = Vec::with_capacity(n);
                {
                    let mut cw = CountingWriter::new(&mut tmp);
                    let _ = zstd.compress(&mut &buf[..n], &mut cw, 3)?;
                }
                let z_csize = tmp.len();

                let (codec, c_size) = if should_compress(n, z_csize, min_gain) {
                    (CodecId::Zstd as u8, z_csize as u64)
                } else {
                    (CodecId::Store as u8, n as u64)
                };

                chunks.push(NewChunk {
                    hash: *hash.as_bytes(),
                    u_size: n as u64,
                    c_size,
                    codec,
                    file_off,
                });

                file_off += n as u64;
            }

            Ok(FilePlan {
                path: src_path.clone(),
                mode: mode_from(&meta),
                mtime: mtime_from(&meta),
                u_size: total_u,
                chunks,
            })
        })
        .collect::<Result<Vec<_>>>()?;

    // ── Manifest planning ────────────────────────────────────────────────────
    let deterministic = opts.map(|o| o.deterministic).unwrap_or(false);
    let created = if deterministic {
        0
    } else {
        OffsetDateTime::now_utc().unix_timestamp()
    };
    let enc = opts.and_then(|o| o.aead_key.as_ref().map(|k| (AeadKey(*k), o.key_salt)));

    let mut chunk_map: HashMap<[u8; 32], u64> = HashMap::new(); // hash → id
    let mut chunk_entries: Vec<ChunkEntry> = Vec::new();
    let mut plans: Vec<ChunkPlan> = Vec::new(); // first occurrences only
    let mut file_entries: Vec<FileEntry> = Vec::new();

    for fp in &file_plans {
        let mut refs = Vec::<ChunkRef>::new();
        for nc in &fp.chunks {
            if let Some(&id) = chunk_map.get(&nc.hash) {
                // duplicate
                refs.push(ChunkRef {
                    id,
                    u_size: nc.u_size,
                });
            } else {
                // first occurrence
                let id = chunk_entries.len() as u64;

                // ciphertext size includes AEAD tag if enabled
                let mut csz = nc.c_size;
                if enc.is_some() {
                    csz += TAG_LEN as u64;
                }

                chunk_entries.push(ChunkEntry {
                    codec: nc.codec,
                    u_size: nc.u_size,
                    c_size: csz,
                    data_off: 0, // patched after layout
                });
                plans.push(ChunkPlan {
                    src: fp.path.clone(),
                    off: nc.file_off,
                    len: nc.u_size,
                    codec: nc.codec,
                });
                chunk_map.insert(nc.hash, id);
                refs.push(ChunkRef {
                    id,
                    u_size: nc.u_size,
                });
            }
        }

        file_entries.push(FileEntry {
            path: rel_display(&fp.path, inputs)?,
            mode: fp.mode,
            mtime: if deterministic { 0 } else { fp.mtime },
            u_size: fp.u_size,
            chunk_refs: refs,
        });
    }

    let dirs_entries: Vec<DirEntry> = dirs
        .iter()
        .map(|d| {
            let md = fs::metadata(d).ok();
            let (m, t) = md
                .map(|md| {
                    (
                        mode_from(&md),
                        if deterministic { 0 } else { mtime_from(&md) },
                    )
                })
                .unwrap_or((0o040755, 0));
            DirEntry {
                path: rel_display(d, inputs).unwrap_or_else(|_| d.display().to_string()),
                mode: m,
                mtime: t,
            }
        })
        .collect();

    let manifest = Manifest {
        files: file_entries,
        dirs: dirs_entries,
        meta: Meta {
            created,
            tool: "arx-core/chunked-alpha".to_string(),
        },
    };

    // ── TailSummary bookkeeping (hashers + totals) ───────────────────────────
    let mut h_manifest = blake3::Hasher::new();
    let mut h_chunktab = blake3::Hasher::new();
    let mut h_data = blake3::Hasher::new();
    let mut total_u: u64 = 0;
    let mut total_c: u64 = 0;

    // ── Manifest (plaintext → optional AEAD) ─────────────────────────────────
    let mut manifest_plain = Vec::new();
    ciborium::ser::into_writer(&manifest, &mut manifest_plain)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
    h_manifest.update(&manifest_plain);

    let enc_enabled = enc.is_some();
    let flags = if enc_enabled { FLAG_ENCRYPTED } else { 0 };

    let (manifest_bytes, manifest_len) = if let Some((ref key, salt)) = enc {
        let nonce = derive_nonce(&salt, Region::Manifest, 0);
        let ct = seal_whole(key, &nonce, b"manifest", &manifest_plain);
        (ct.clone(), ct.len() as u64)
    } else {
        (manifest_plain, /*len*/ 0) // set below
    };
    let manifest_len = if enc_enabled {
        manifest_len
    } else {
        manifest_bytes.len() as u64
    };

    // ── Compute layout BEFORE serializing the table ──────────────────────────
    let chunk_count = chunk_entries.len() as u64;
    let pt_table_len = (chunk_entries.len() * ENTRY_SIZE) as u64;
    let table_len = if enc_enabled {
        pt_table_len + TAG_LEN as u64 // one AEAD tag for the whole table region
    } else {
        pt_table_len
    };

    let chunk_table_off = HEADER_LEN + manifest_len;
    let data_off = chunk_table_off + table_len;

    // Patch data_offs (absolute file offsets into the DATA ciphertext/plaintext region)
    let mut cursor = data_off;
    for ce in &mut chunk_entries {
        ce.data_off = cursor;
        cursor += ce.c_size; // ce.c_size already includes per-chunk AEAD tag when enabled
    }

    // Now serialize **patched** table to plaintext, then encrypt if needed
    let mut table_plain = Vec::with_capacity(pt_table_len as usize);
    write_table(&mut table_plain, &chunk_entries)?;
    debug_assert_eq!(table_plain.len() as u64, pt_table_len);
    h_chunktab.update(&table_plain);

    let (table_bytes, table_len_check) = if let Some((ref key, salt)) = enc {
        let nonce = derive_nonce(&salt, Region::ChunkTable, 0);
        let ct = seal_whole(key, &nonce, b"chunktab", &table_plain);
        (ct, pt_table_len + TAG_LEN as u64)
    } else {
        (table_plain, pt_table_len)
    };
    debug_assert_eq!(table_len_check, table_len);

    // ── Write superblock stub + regions ──────────────────────────────────────
    let mut out_f = File::create(out)?;
    Superblock {
        version: VERSION,
        manifest_len: 0,
        chunk_table_off: 0,
        chunk_count: 0,
        data_off: 0,
        flags: 0,
    }
    .write_to(&mut out_f)?;

    // Manifest
    out_f.seek(SeekFrom::Start(HEADER_LEN))?;
    out_f.write_all(&manifest_bytes)?;

    // Chunk table (with correct data_offs)
    out_f.seek(SeekFrom::Start(chunk_table_off))?;
    out_f.write_all(&table_bytes)?;

    // ── Data region ──────────────────────────────────────────────────────────
    let zstd_w = ZstdCompressor;
    let mut io_buf = vec![0u8; 1 << 16];

    for (i, ce) in chunk_entries.iter().enumerate() {
        let plan = &plans[i];

        out_f.seek(SeekFrom::Start(ce.data_off))?;
        let mut src = File::open(&plan.src)?;
        src.seek(SeekFrom::Start(plan.off))?;

        let mut plain = Vec::with_capacity(plan.len as usize);
        let mut left = plan.len;
        while left > 0 {
            let n = io_buf.len().min(left as usize);
            let k = src.read(&mut io_buf[..n])?;
            if k == 0 {
                break;
            }
            plain.extend_from_slice(&io_buf[..k]);
            left -= k as u64;
        }

        // Compress/store -> yields COMPRESSED PLAINTEXT bytes
        let comp = match plan.codec {
            x if x == CodecId::Store as u8 => plain,
            x if x == CodecId::Zstd as u8 => {
                let mut tmp = std::io::Cursor::new(Vec::<u8>::new());
                let mut cw = CountingWriter::new(&mut tmp);
                zstd_w.compress(&mut &plain[..], &mut cw, 3)?;
                tmp.into_inner()
            }
            _ => {
                return Err(
                    std::io::Error::new(std::io::ErrorKind::Other, "unknown codec id").into(),
                );
            }
        };

        // Tail data hash + totals
        h_data.update(&comp);
        total_u = total_u.saturating_add(plan.len);
        total_c = total_c.saturating_add(comp.len() as u64);

        // AEAD (if enabled) and write
        if let Some((ref key, salt)) = enc {
            let nonce = derive_nonce(&salt, Region::ChunkData, i as u64); // id == index
            let ct = seal_whole(key, &nonce, b"chunk", &comp);
            debug_assert_eq!(ct.len() as u64, ce.c_size);
            out_f.write_all(&ct)?;
        } else {
            debug_assert_eq!(comp.len() as u64, ce.c_size);
            out_f.write_all(&comp)?;
        }
    }

    // ── Rewrite real Superblock ──────────────────────────────────────────────
    out_f.seek(SeekFrom::Start(0))?;
    Superblock {
        version: VERSION,
        manifest_len,
        chunk_table_off,
        chunk_count,
        data_off,
        flags,
    }
    .write_to(&mut out_f)?;

    // ── Tail Summary at EOF ──────────────────────────────────────────────────
    out_f.seek(SeekFrom::End(0))?;
    let tail = TailSummary {
        manifest_blake3: *h_manifest.finalize().as_bytes(),
        chunktab_blake3: *h_chunktab.finalize().as_bytes(),
        data_blake3: *h_data.finalize().as_bytes(),
        total_u,
        total_c,
    };
    tail.write_to(&mut out_f)?;
    out_f.flush()?;

    Ok(())
}
