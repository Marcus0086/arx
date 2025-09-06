use crate::container::manifest::Manifest;
use crate::container::superblock::Superblock;
use crate::error::ArxError;
use std::fs::{self, File};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::Path;

pub fn extract(archive: &Path, dest: &Path) -> Result<(), ArxError> {
    let mut f = File::open(archive)?;
    let sb = Superblock::read_from(&mut f)?;
    let mut man_buf = vec![0u8; sb.manifest_len as usize];
    f.read_exact(&mut man_buf)?;
    let manifest: Manifest = ciborium::de::from_reader(&man_buf[..])
        .map_err(|e| ArxError::Format(format!("manifest decode: {e}")))?;

    for d in &manifest.dirs {
        let p = dest.join(&d.path);
        fs::create_dir_all(&p)?;
        // (mode/mtime restore later)
    }
    let mut buf = vec![0u8; 1 << 16];
    for fe in &manifest.files {
        let outp = dest.join(&fe.path);
        if let Some(parent) = outp.parent() {
            fs::create_dir_all(parent)?;
        }
        let mut out = File::create(&outp)?;
        f.seek(SeekFrom::Start(fe.data_off))?;
        let mut left = fe.size;
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
    Ok(())
}
