use std::io::{Read, Write};

pub const MAGIC: &[u8; 6] = b"ARXALP"; // alpha marker
pub const VERSION: u16 = 1;

#[derive(Debug, Clone, Copy)]
pub struct Superblock {
    pub version: u16,
    /// Byte length of the manifest (CBOR)
    pub manifest_len: u64,
    /// Absolute file offset where the data section starts (manifest_end)
    pub data_off: u64,
}

impl Superblock {
    pub fn write_to(&self, mut w: impl Write) -> std::io::Result<()> {
        w.write_all(MAGIC)?;
        w.write_all(&self.version.to_le_bytes())?;
        w.write_all(&self.manifest_len.to_le_bytes())?;
        w.write_all(&self.data_off.to_le_bytes())?;
        Ok(())
    }

    pub fn read_from(mut r: impl Read) -> std::io::Result<Self> {
        let mut magic = [0u8; 6];
        r.read_exact(&mut magic)?;
        if &magic != MAGIC {
            return Err(std::io::ErrorKind::InvalidData.into());
        }
        let mut v = [0u8; 2];
        r.read_exact(&mut v)?;
        let version = u16::from_le_bytes(v);
        let mut ml = [0u8; 8];
        r.read_exact(&mut ml)?;
        let manifest_len = u64::from_le_bytes(ml);
        let mut doff = [0u8; 8];
        r.read_exact(&mut doff)?;
        let data_off = u64::from_le_bytes(doff);
        Ok(Self {
            version,
            manifest_len,
            data_off,
        })
    }
}
