use crate::codec::store::Store;
use crate::codec::zstdc::ZstdCompressor;
use crate::codec::{CodecId, Compressor};
use crate::container::manifest::Manifest;
use crate::container::superblock::Superblock;
use crate::error::Result;
use std::fs::{self, File};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

pub fn extract(archive: &Path, dest: &Path) -> Result<()> {
    let mut f = File::open(archive)?;
    let sb = Superblock::read_from(&mut f)?;

    // seek to manifest and read it
    f.seek(SeekFrom::Start(24))?;
    let mut man_buf = vec![0u8; sb.manifest_len as usize];
    f.read_exact(&mut man_buf)?;
    let manifest: Manifest = ciborium::de::from_reader(&man_buf[..])
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;

    // create directories first
    for d in &manifest.dirs {
        let p = safe_join(dest, &d.path)?;
        fs::create_dir_all(&p)?;
        // TODO: restore d.mode / d.mtime if desired
    }

    let zstd = ZstdCompressor;
    let store = Store;

    // extract files (streamed)
    for fe in &manifest.files {
        let outp = safe_join(dest, &fe.path)?;
        if let Some(parent) = outp.parent() {
            fs::create_dir_all(parent)?;
        }
        let mut out = File::create(&outp)?;

        // position at file's data and wrap a limited reader of c_size bytes
        f.seek(SeekFrom::Start(fe.data_off))?;
        let mut limited = LimitedReader::new(&mut f, fe.c_size);

        match CodecId::from_u8(fe.codec) {
            Some(CodecId::Store) => copy_stream(&mut limited, &mut out)?,
            Some(CodecId::Zstd) => zstd.decompress(&mut limited, &mut out)?,
            None => {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!("unknown codec id {}", fe.codec),
                )
                .into());
            }
        };

        if out.metadata()?.len() != fe.u_size {
            return Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!(
                    "uncompressed size mismatch: {} != {}",
                    out.metadata()?.len(),
                    fe.u_size
                ),
            )
            .into());
        }

        // TODO: restore fe.mode / fe.mtime if desired
    }
    Ok(())
}

// ---- helpers ----

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
        let to_read = buf.len().min(self.remaining as usize);
        let n = self.inner.read(&mut buf[..to_read])?;
        self.remaining -= n as u64;
        Ok(n)
    }
}

fn copy_stream<R: Read, W: Write>(mut r: R, mut w: W) -> std::io::Result<u64> {
    let mut buf = [0u8; 1 << 16];
    let mut total = 0u64;
    loop {
        let n = r.read(&mut buf)?;
        if n == 0 {
            break;
        }
        w.write_all(&buf[..n])?;
        total += n as u64;
    }
    Ok(total)
}

fn safe_join(root: &Path, rel: &str) -> Result<PathBuf> {
    let p = Path::new(rel);
    // reject absolute paths and traversal
    if p.is_absolute() || rel.contains("../") || rel.contains("..\\") {
        return Err(
            std::io::Error::new(std::io::ErrorKind::Other, format!("unsafe path: {rel}")).into(),
        );
    }
    Ok(root.join(p))
}

// If you don't have this on CodecId yet:
impl CodecId {
    pub fn from_u8(v: u8) -> Option<Self> {
        match v {
            x if x == CodecId::Store as u8 => Some(CodecId::Store),
            x if x == CodecId::Zstd as u8 => Some(CodecId::Zstd),
            _ => None,
        }
    }
}
