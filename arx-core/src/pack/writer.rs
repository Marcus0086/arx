use crate::codec::store::Store;
use crate::codec::zstdc::ZstdCompressor;
use crate::codec::{CodecId, Compressor};
use crate::container::manifest::{DirEntry, FileEntry, Manifest, Meta};
use crate::container::superblock::Superblock;
use crate::error::Result;
use rayon::prelude::*;
use std::fs::{self, File};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use tempfile::NamedTempFile;
use time::OffsetDateTime;
use walkdir::WalkDir;

#[derive(Clone)]
pub enum Encryption {/* reserved */}

#[derive(Clone, Default)]
pub struct PackOptions {
    pub deterministic: bool,
}

pub fn pack(inputs: &[&Path], out: &Path, opts: Option<&PackOptions>) -> Result<()> {
    // 1) collect dirs/files (stable)
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

    // 2) parallel compress each file to a temp file
    struct TmpOut {
        fe: FileEntry,
        tmp: NamedTempFile, // holds compressed or stored bytes
    }

    let zstd = ZstdCompressor;
    let store = Store;

    let outs: Vec<TmpOut> = files
        .par_iter()
        .map(|src_path| -> Result<TmpOut> {
            let meta = fs::metadata(src_path)?;
            let u = meta.len();
            let m = mode_from(&meta);
            let t = mtime_from(&meta);

            // compress (try zstd then maybe fallback to store)
            let mut tmp = NamedTempFile::new()?;
            {
                let mut in_f = File::open(src_path)?;
                let mut counting = CountingWriter::new(&mut tmp);
                let _uncomp_written = zstd.compress(&mut in_f, &mut counting, 3)?;
                let c_size = counting.bytes();

                if (c_size as f64) > (u as f64 * 0.95) {
                    // redo as STORE
                    tmp.as_file().set_len(0)?;
                    tmp.as_file().seek(SeekFrom::Start(0))?;
                    let mut in_f2 = File::open(src_path)?;
                    let mut counting2 = CountingWriter::new(&mut tmp);
                    let _ = store.compress(&mut in_f2, &mut counting2, 0)?;
                    let c_size2 = counting2.bytes();
                    Ok(TmpOut {
                        fe: FileEntry {
                            path: rel_display(src_path, inputs)?,
                            mode: m,
                            mtime: t,
                            u_size: u,
                            c_size: c_size2,
                            codec: CodecId::Store as u8,
                            data_off: 0,
                        },
                        tmp,
                    })
                } else {
                    Ok(TmpOut {
                        fe: FileEntry {
                            path: rel_display(src_path, inputs)?,
                            mode: m,
                            mtime: t,
                            u_size: u,
                            c_size,
                            codec: CodecId::Zstd as u8,
                            data_off: 0,
                        },
                        tmp,
                    })
                }
            }
        })
        .collect::<Result<Vec<_>>>()?;

    // 3) build manifest from outs (files) + dirs
    let deterministic = opts.map(|o| o.deterministic).unwrap_or(false);
    let created = if deterministic {
        0
    } else {
        OffsetDateTime::now_utc().unix_timestamp()
    };

    let mut manifest = Manifest {
        files: outs.iter().map(|o| o.fe.clone()).collect(),
        dirs: dirs
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
            .collect(),
        meta: Meta {
            created,
            tool: "arx-core/alpha".into(),
        },
    };

    if deterministic {
        // zero mtimes in files too
        for fe in manifest.files.iter_mut() {
            fe.mtime = 0;
        }
    }

    // 4) compute manifest_len + data_off; offsets from c_size (compressed)
    let mut manifest_buf = Vec::new();
    let mut manifest_len: u64 = 0;
    let mut data_off: u64 = 24; // superblock size

    loop {
        data_off = 24 + manifest_len;
        let mut cursor = data_off;
        for fe in manifest.files.iter_mut() {
            fe.data_off = cursor;
            cursor += fe.c_size;
        }
        manifest_buf.clear();
        ciborium::ser::into_writer(&manifest, &mut manifest_buf)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        let new_len = manifest_buf.len() as u64;
        if new_len == manifest_len {
            break;
        }
        manifest_len = new_len;
    }

    // 5) write archive (SB placeholder → manifest → data → SB finalize)
    let mut out_f = File::create(out)?;
    Superblock {
        version: crate::container::superblock::VERSION,
        manifest_len: 0,
        data_off: 0,
    }
    .write_to(&mut out_f)?;
    out_f.seek(SeekFrom::Start(24))?;
    out_f.write_all(&manifest_buf)?;
    debug_assert_eq!(out_f.stream_position()?, data_off);

    // stream each temp file at its assigned offset
    let mut buf = vec![0u8; 1 << 16];
    for (fe, o) in manifest.files.iter().zip(outs.iter()) {
        out_f.seek(SeekFrom::Start(fe.data_off))?;
        let mut tf = o.tmp.reopen()?; // fresh handle at pos 0
        loop {
            let n = tf.read(&mut buf)?;
            if n == 0 {
                break;
            }
            out_f.write_all(&buf[..n])?;
        }
    }

    out_f.seek(SeekFrom::Start(0))?;
    Superblock {
        version: crate::container::superblock::VERSION,
        manifest_len,
        data_off,
    }
    .write_to(&mut out_f)?;

    Ok(())
}

struct CountingWriter<'a, W: Write> {
    inner: &'a mut W,
    n: u64,
}
impl<'a, W: Write> CountingWriter<'a, W> {
    fn new(inner: &'a mut W) -> Self {
        Self { inner, n: 0 }
    }
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

// helpers: mode_from, mtime_from, rel_display (unchanged)

// helpers
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
