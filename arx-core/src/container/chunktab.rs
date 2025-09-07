use crate::error::Result;
use std::env;
use std::io::{self, Write};

pub const ENTRY_SIZE: usize = 32;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChunkEntry {
    pub codec: u8,
    pub u_size: u64,
    pub c_size: u64,
    pub data_off: u64,
}

pub fn write_table(mut w: impl Write, entries: &[ChunkEntry]) -> Result<()> {
    let mut buf = [0u8; ENTRY_SIZE];
    for e in entries {
        buf[0] = e.codec;
        for b in &mut buf[1..8] {
            *b = 0;
        }
        buf[8..16].copy_from_slice(&e.u_size.to_le_bytes());
        buf[16..24].copy_from_slice(&e.c_size.to_le_bytes());
        buf[24..32].copy_from_slice(&e.data_off.to_le_bytes());
        w.write_all(&buf)?;
    }
    Ok(())
}

#[inline]
fn le64(x: &[u8]) -> u64 {
    u64::from_le_bytes(x.try_into().unwrap())
}

pub fn read_table_from_slice(buf: &[u8], count: u64) -> std::io::Result<Vec<ChunkEntry>> {
    let need = count as usize * ENTRY_SIZE;
    if buf.len() != need {
        return Err(std::io::Error::new(
            std::io::ErrorKind::UnexpectedEof,
            format!(
                "chunk table size mismatch: got {} bytes, expected {}",
                buf.len(),
                need
            ),
        ));
    }

    let mut out = Vec::with_capacity(count as usize);
    let mut off = 0usize;
    for _ in 0..count {
        let e = &buf[off..off + ENTRY_SIZE];
        // Layout: [0]=codec (u8), [1..8]=pad, [8..16]=u_size, [16..24]=c_size, [24..32]=data_off
        let codec = e[0];
        let u_size = le64(&e[8..16]);
        let c_size = le64(&e[16..24]);
        let data_off = le64(&e[24..32]);
        out.push(ChunkEntry {
            codec,
            u_size,
            c_size,
            data_off,
        });
        off += ENTRY_SIZE;
    }
    Ok(out)
}

pub fn read_table(r: &mut &[u8], count: u64) -> Result<Vec<ChunkEntry>> {
    let dbg = env::var_os("ARX_DEBUG_LIST").is_some();
    let need_u64 = count
        .checked_mul(ENTRY_SIZE as u64)
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "chunk table size overflow"))?;
    let need = need_u64 as usize;

    if r.len() < need {
        if dbg {
            eprintln!(
                "[DBG] chunktab: insufficient bytes: have={}, need={} (entries={} * {})",
                r.len(),
                need,
                count,
                ENTRY_SIZE
            );
        }
        return Err(
            io::Error::new(io::ErrorKind::UnexpectedEof, "failed to fill whole buffer").into(),
        );
    }

    let slice = &r[..need];
    let mut out = Vec::with_capacity(count as usize);
    let mut off = 0usize;

    for i in 0..(count as usize) {
        let codec = slice[off];
        let u_size = u64::from_le_bytes(slice[off + 8..off + 16].try_into().unwrap());
        let c_size = u64::from_le_bytes(slice[off + 16..off + 24].try_into().unwrap());
        let data_off = u64::from_le_bytes(slice[off + 24..off + 32].try_into().unwrap());

        if dbg {
            eprintln!("[DBG] CE[{i}]: codec={codec} u={u_size} c={c_size} off={data_off}");
        }

        out.push(ChunkEntry {
            codec,
            u_size,
            c_size,
            data_off,
        });
        off += ENTRY_SIZE;
    }

    *r = &r[need..]; // advance
    Ok(out)
}
