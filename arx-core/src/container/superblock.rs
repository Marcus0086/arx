use std::io::{Read, Write};

pub const MAGIC: &[u8; 6] = b"ARXALP"; // alpha marker
pub const VERSION: u16 = 3;

// 6 bytes for magic
// 2 bytes for version
// 8 bytes for manifest_len
// 8 bytes for chunk_table_off
// 8 bytes for chunk_count
// 8 bytes for data_off
// 8 bytes for flags
pub const HEADER_LEN: u64 = 48; // 6 + 2 + 8 + 8 + 8 + 8 + 8
pub const FLAG_ENCRYPTED: u64 = 1 << 0;
#[derive(Debug, Clone, Copy)]
pub struct Superblock {
    pub version: u16,
    /// Byte length of the manifest (CBOR)
    pub manifest_len: u64,
    pub chunk_table_off: u64,
    pub chunk_count: u64,
    /// Absolute file offset where the data section starts (manifest_end)
    pub data_off: u64,
    pub flags: u64,
}

impl Superblock {
    pub fn write_to(&self, mut w: impl Write) -> std::io::Result<()> {
        w.write_all(MAGIC)?;
        w.write_all(&self.version.to_le_bytes())?;
        w.write_all(&self.manifest_len.to_le_bytes())?;
        w.write_all(&self.chunk_table_off.to_le_bytes())?;
        w.write_all(&self.chunk_count.to_le_bytes())?;
        w.write_all(&self.data_off.to_le_bytes())?;
        w.write_all(&self.flags.to_le_bytes())?;
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

        let mut cto = [0u8; 8];
        r.read_exact(&mut cto)?;
        let chunk_table_off = u64::from_le_bytes(cto);

        let mut cc = [0u8; 8];
        r.read_exact(&mut cc)?;
        let chunk_count = u64::from_le_bytes(cc);

        let mut doff = [0u8; 8];
        r.read_exact(&mut doff)?;
        let data_off = u64::from_le_bytes(doff);

        let mut flags = [0u8; 8];
        r.read_exact(&mut flags)?;
        let flags = u64::from_le_bytes(flags);

        Ok(Self {
            version,
            manifest_len,
            chunk_table_off,
            chunk_count,
            data_off,
            flags,
        })
    }
}
