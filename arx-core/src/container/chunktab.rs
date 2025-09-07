use std::io::{Read, Write};

/// Single chunk metadata (dedup target)
#[derive(Debug, Clone)]
pub struct ChunkEntry {
    pub hash: [u8; 32], // BLAKE3 of uncompressed bytes
    pub codec: u8,      // 0: STORE, 1: ZSTD
    pub u_size: u64,
    pub c_size: u64,
    pub data_off: u64, // absolute archive offset where chunk bytes start
}

// 32(hash) + 1(codec) + 7(pad) + 8+8+8 = 64 bytes (nicely aligned)
pub const ENTRY_SIZE: usize = 64;

pub fn write_table(mut w: impl Write, entries: &[ChunkEntry]) -> std::io::Result<()> {
    let pad = [0u8; 7];
    for e in entries {
        w.write_all(&e.hash)?;
        w.write_all(&[e.codec])?;
        w.write_all(&pad)?;
        w.write_all(&e.u_size.to_le_bytes())?;
        w.write_all(&e.c_size.to_le_bytes())?;
        w.write_all(&e.data_off.to_le_bytes())?;
    }
    Ok(())
}

pub fn read_table(mut r: impl Read, count: u64) -> std::io::Result<Vec<ChunkEntry>> {
    let mut v = Vec::with_capacity(count as usize);
    for _ in 0..count {
        let mut hash = [0u8; 32];
        r.read_exact(&mut hash)?;
        let mut c = [0u8; 1];
        r.read_exact(&mut c)?;
        let mut pad = [0u8; 7];
        r.read_exact(&mut pad)?;
        let mut b8 = [0u8; 8];
        r.read_exact(&mut b8)?;
        let u_size = u64::from_le_bytes(b8);
        r.read_exact(&mut b8)?;
        let c_size = u64::from_le_bytes(b8);
        r.read_exact(&mut b8)?;
        let data_off = u64::from_le_bytes(b8);
        v.push(ChunkEntry {
            hash,
            codec: c[0],
            u_size,
            c_size,
            data_off,
        });
    }
    Ok(v)
}
