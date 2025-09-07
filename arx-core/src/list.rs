use crate::container::manifest::Manifest;
use crate::container::superblock::Superblock;
use crate::error::Result;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;

pub fn list(archive: &Path) -> Result<()> {
    // 1) open & read superblock
    let mut f = File::open(archive)?;
    let sb = Superblock::read_from(&mut f)?; // if you don't have this, see helper below

    // 2) read manifest bytes
    f.seek(SeekFrom::Start(24))?;
    let mut mbytes = vec![0u8; sb.manifest_len as usize];
    f.read_exact(&mut mbytes)?;

    // 3) decode CBOR → Manifest
    let manifest: Manifest = ciborium::de::from_reader(&mbytes[..])
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;

    for fe in &manifest.files {
        println!("{}  {} bytes  off={}", fe.path, fe.u_size, fe.data_off);
    }

    Ok(())
}
