use crate::container::manifest::Manifest;
use crate::container::superblock::Superblock;
use crate::error::ArxError;
use std::fs::File;
use std::io::Read;
use std::path::Path;

pub fn list(archive: &Path) -> Result<(), ArxError> {
    let mut f = File::open(archive)?;
    let sb = Superblock::read_from(&mut f)?;
    let mut man_buf = vec![0u8; sb.manifest_len as usize];
    f.read_exact(&mut man_buf)?;
    let manifest: Manifest = ciborium::de::from_reader(&man_buf[..])
        .map_err(|e| ArxError::Format(format!("manifest decode: {e}")))?;
    for fe in &manifest.files {
        println!("{}  {} bytes  off={}", fe.path, fe.size, fe.data_off);
    }
    Ok(())
}
