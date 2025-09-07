use std::io::{Read, Seek, SeekFrom, Write};

pub const TAIL_MAGIC: [u8; 8] = *b"ARXTAIL\0";
pub const TAIL_LEN: u64 = 120;

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct TailSummary {
    pub manifest_blake3: [u8; 32],
    pub chunktab_blake3: [u8; 32],
    pub data_blake3: [u8; 32],
    pub total_u: u64,
    pub total_c: u64,
}

impl TailSummary {
    pub fn write_to<W: Write>(self, mut w: W) -> std::io::Result<()> {
        w.write_all(&TAIL_MAGIC)?;
        w.write_all(&self.manifest_blake3)?;
        w.write_all(&self.chunktab_blake3)?;
        w.write_all(&self.data_blake3)?;
        w.write_all(&self.total_u.to_le_bytes())?;
        w.write_all(&self.total_c.to_le_bytes())?;
        Ok(())
    }

    pub fn read_from<R: Read>(mut r: R) -> std::io::Result<Self> {
        let mut magic = [0u8; 8];
        r.read_exact(&mut magic)?;
        if magic != TAIL_MAGIC {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "bad tail magic",
            ));
        }
        let mut t = TailSummary::default();
        r.read_exact(&mut t.manifest_blake3)?;
        r.read_exact(&mut t.chunktab_blake3)?;
        r.read_exact(&mut t.data_blake3)?;
        let mut buf8 = [0u8; 8];
        r.read_exact(&mut buf8)?;
        t.total_u = u64::from_le_bytes(buf8);
        r.read_exact(&mut buf8)?;
        t.total_c = u64::from_le_bytes(buf8);
        Ok(t)
    }
}

/// Locate the Tail by reading the last 120 bytes of the file.
pub fn read_tail_at_eof<F: Read + Seek>(f: &mut F) -> std::io::Result<TailSummary> {
    let len = f.seek(SeekFrom::End(0))?;
    if len < TAIL_LEN {
        return Err(std::io::Error::new(
            std::io::ErrorKind::UnexpectedEof,
            "file too small for tail",
        ));
    }
    f.seek(SeekFrom::End(-(TAIL_LEN as i64)))?;
    TailSummary::read_from(f)
}
