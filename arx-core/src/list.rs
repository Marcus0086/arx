use crate::container::chunktab::read_table;
use crate::container::manifest::Manifest;
use crate::container::superblock::{HEADER_LEN, Superblock};
use crate::error::Result;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;

pub fn list(archive: &Path) -> Result<()> {
    let mut f = File::open(archive)?;
    let sb = Superblock::read_from(&mut f)?;

    f.seek(SeekFrom::Start(HEADER_LEN))?;
    let mut mbytes = vec![0u8; sb.manifest_len as usize];
    f.read_exact(&mut mbytes)?;
    let manifest: Manifest = ciborium::de::from_reader(&mbytes[..])
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;

    // Optional: load table to compute compressed totals per file
    f.seek(SeekFrom::Start(sb.chunk_table_off))?;
    let table = read_table(&mut f, sb.chunk_count)?;

    for fe in &manifest.files {
        let mut c_sum = 0u64;
        for c in &fe.chunk_refs {
            c_sum += table[c.id as usize].c_size;
        }
        println!(
            "{}  u={}  c={}  chunks={}",
            fe.path,
            fe.u_size,
            c_sum,
            fe.chunk_refs.len()
        );
    }
    Ok(())
}
