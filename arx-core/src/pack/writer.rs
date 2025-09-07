use crate::chunking::fastcdc::{ChunkParams, StreamingChunker};
use crate::codec::store::Store;
use crate::codec::zstdc::ZstdCompressor;
use crate::codec::{CodecId, Compressor};
use crate::container::chunktab::{ChunkEntry, ENTRY_SIZE, write_table};
use crate::container::manifest::{ChunkRef, DirEntry, FileEntry, Manifest, Meta};
use crate::container::superblock::{HEADER_LEN, Superblock, VERSION};
use crate::error::Result;
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
    /// Only accept compression if it saves at least this fraction.
    /// e.g. 0.05 means "compress only if >=5% smaller than STORE".
    pub min_gain: f32, // default 0.05 if left as 0.0
}

/// Small Write adapter that counts bytes written
struct CountingWriter<'a, W: Write> {
    inner: &'a mut W,
    n: u64,
}
impl<'a, W: Write> CountingWriter<'a, W> {
    fn new(inner: &'a mut W) -> Self {
        Self { inner, n: 0 }
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

struct LimitedReader<'a, R: Read> {
    inner: &'a mut R,
    remaining: u64,
}
impl<'a, R: Read> LimitedReader<'a, R> {
    fn new(inner: &'a mut R, limit: u64) -> Self {
        Self {
            inner,
            remaining: limit,
        }
    }
}
impl<'a, R: Read> Read for LimitedReader<'a, R> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        if self.remaining == 0 {
            return Ok(0);
        }
        let want = buf.len().min(self.remaining as usize);
        let n = self.inner.read(&mut buf[..want])?;
        self.remaining -= n as u64;
        Ok(n)
    }
}

fn mode_from(_md: &std::fs::Metadata) -> u32 {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        _md.permissions().mode()
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
    // find first root that is a prefix; else emit as relative string
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
    // true if (u - c) >= u * min_gain  ⇔  c <= u * (1 - min_gain)
    (u as f64 - c as f64) >= (u as f64 * min_gain as f64)
}

#[derive(Clone)]
struct NewChunk {
    hash: [u8; 32],
    u_size: u64,
    c_size: u64,
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

/// Plan for first occurrence of a unique chunk (used during write pass)
struct ChunkPlan {
    src: PathBuf,
    off: u64,
    len: u64,
    codec: u8,
}

pub fn pack(inputs: &[&Path], out: &Path, opts: Option<&PackOptions>) -> Result<()> {
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
            // (symlinks skipped in alpha)
        }
    }
    dirs.sort();
    files.sort();

    let min_gain = effective_min_gain(opts);
    let params = ChunkParams {
        min: 64 * 1024,
        avg: 256 * 1024,
        max: 1 * 1024 * 1024,
    }; // ~256 KiB target chunk size
    let zstd = ZstdCompressor;
    let _store = Store; // not used for trial; STORE size == u_size

    let file_plans: Vec<FilePlan> = files
        .par_iter() // In parallel, each file independent
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

                // Hash chunk (uncompressed)
                let hash = blake3::hash(&buf[..n]);

                // Trial compress to measure compressed size
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

    let deterministic = opts.map(|o| o.deterministic).unwrap_or(false);
    let created = if deterministic {
        0
    } else {
        OffsetDateTime::now_utc().unix_timestamp()
    };

    let mut chunk_map: HashMap<[u8; 32], u64> = HashMap::new(); // hash → id
    let mut chunk_entries: Vec<ChunkEntry> = Vec::new();
    let mut plans: Vec<ChunkPlan> = Vec::new(); // only for first occurrences (unique chunks)
    let mut file_entries: Vec<FileEntry> = Vec::new();

    for fp in &file_plans {
        let mut refs = Vec::<ChunkRef>::new();
        for nc in &fp.chunks {
            if let Some(&id) = chunk_map.get(&nc.hash) {
                // duplicate chunk: just reference
                refs.push(ChunkRef {
                    id,
                    u_size: nc.u_size,
                });
            } else {
                let id = chunk_entries.len() as u64;
                chunk_entries.push(ChunkEntry {
                    hash: nc.hash,
                    codec: nc.codec,
                    u_size: nc.u_size,
                    c_size: nc.c_size,
                    data_off: 0, // patch after we compute layout
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

    let mut manifest_buf = Vec::new();
    ciborium::ser::into_writer(&manifest, &mut manifest_buf)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
    let manifest_len = manifest_buf.len() as u64;

    let chunk_table_off = HEADER_LEN + manifest_len;
    let chunk_count = chunk_entries.len() as u64;
    let chunk_table_len = (chunk_count as usize * ENTRY_SIZE) as u64;

    let mut cursor = chunk_table_off + chunk_table_len;
    for ce in &mut chunk_entries {
        ce.data_off = cursor;
        cursor += ce.c_size;
    }
    let data_off = chunk_table_off + chunk_table_len;

    let mut out_f = File::create(out)?;

    Superblock {
        version: VERSION,
        manifest_len: 0,
        chunk_table_off: 0,
        chunk_count: 0,
        data_off: 0,
    }
    .write_to(&mut out_f)?;

    // manifest
    out_f.seek(SeekFrom::Start(HEADER_LEN))?;
    out_f.write_all(&manifest_buf)?;

    // chunk table
    out_f.seek(SeekFrom::Start(chunk_table_off))?;
    write_table(&mut out_f, &chunk_entries)?;

    // chunk data (streaming). Re-read slices from original files and compress to out.
    let zstd_w = ZstdCompressor;
    let mut io_buf = vec![0u8; 1 << 16];

    for (i, ce) in chunk_entries.iter().enumerate() {
        let plan = &plans[i];

        out_f.seek(SeekFrom::Start(ce.data_off))?;
        let mut src = File::open(&plan.src)?;
        src.seek(SeekFrom::Start(plan.off))?;

        match plan.codec {
            x if x == CodecId::Store as u8 => {
                // STORE: copy exactly `plan.len` bytes
                let mut left = plan.len;
                while left > 0 {
                    let n = io_buf.len().min(left as usize);
                    let k = src.read(&mut io_buf[..n])?;
                    if k == 0 {
                        break;
                    }
                    out_f.write_all(&io_buf[..k])?;
                    left -= k as u64;
                }
            }
            x if x == CodecId::Zstd as u8 => {
                // Zstd: compress this chunk region into out_f
                // Use our trait (keeps single-threaded for deterministic-by-default)
                let mut limited = LimitedReader::new(&mut src, plan.len);
                zstd_w.compress(&mut limited, &mut out_f, 3)?;
            }
            _ => {
                return Err(
                    std::io::Error::new(std::io::ErrorKind::Other, "unknown codec id").into(),
                );
            }
        }
    }

    // finalize superblock
    out_f.seek(SeekFrom::Start(0))?;
    Superblock {
        version: VERSION,
        manifest_len,
        chunk_table_off,
        chunk_count,
        data_off,
    }
    .write_to(&mut out_f)?;

    Ok(())
}
