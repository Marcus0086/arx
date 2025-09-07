use crate::codec::CodecId;
use crate::container::chunktab::{ChunkEntry, read_table};
use crate::container::manifest::Manifest;
use crate::container::superblock::{HEADER_LEN, Superblock};
use crate::error::Result;
use std::fs::{self, File};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

pub fn extract(archive: &Path, dest: &Path) -> Result<()> {
    let mut f = File::open(archive)?;
    let sb = Superblock::read_from(&mut f)?;

    // manifest
    f.seek(SeekFrom::Start(HEADER_LEN))?;
    let mut man_buf = vec![0u8; sb.manifest_len as usize];
    f.read_exact(&mut man_buf)?;
    let manifest: Manifest = ciborium::de::from_reader(&man_buf[..])
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;

    // chunk table
    f.seek(SeekFrom::Start(sb.chunk_table_off))?;
    let table = read_table(&mut f, sb.chunk_count)?;

    // dirs first
    for d in &manifest.dirs {
        let p = safe_join(dest, &d.path)?;
        fs::create_dir_all(&p)?;
    }

    let mut buf = vec![0u8; 1 << 16];

    // files from chunk refs
    for fe in &manifest.files {
        let outp = safe_join(dest, &fe.path)?;
        if let Some(parent) = outp.parent() {
            fs::create_dir_all(parent)?;
        }
        let mut out = File::create(&outp)?;
        for cref in &fe.chunk_refs {
            let ce: &ChunkEntry = &table[cref.id as usize];
            f.seek(SeekFrom::Start(ce.data_off))?;
            let mut left = ce.c_size;

            match ce.codec {
                x if x == CodecId::Store as u8 => {
                    while left > 0 {
                        let n = buf.len().min(left as usize);
                        let k = f.read(&mut buf[..n])?;
                        if k == 0 {
                            break;
                        }
                        out.write_all(&buf[..k])?;
                        left -= k as u64;
                    }
                }
                x if x == CodecId::Zstd as u8 => {
                    // decode this chunk into out
                    let mut decoder =
                        zstd::stream::read::Decoder::new(LimitedReader::new(&mut f, ce.c_size))?;
                    loop {
                        let k = decoder.read(&mut buf)?;
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
        // Optional: size check
        if out.metadata()?.len() != fe.u_size {
            return Err(
                std::io::Error::new(std::io::ErrorKind::Other, "extracted size mismatch").into(),
            );
        }
    }

    Ok(())
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
        let to_read = buf.len().min(self.remaining as usize);
        let n = self.inner.read(&mut buf[..to_read])?;
        self.remaining -= n as u64;
        Ok(n)
    }
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
