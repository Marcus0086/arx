use crate::error::Result;
use std::env;
use std::io::{self, Write};

/// Entry size for v4+ archives (64 bytes — includes blake3 hash).
pub const ENTRY_SIZE: usize = 64;
/// Entry size for v3 archives (32 bytes — no blake3 hash).
pub const ENTRY_SIZE_V3: usize = 32;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChunkEntry {
    pub codec: u8,
    pub u_size: u64,
    pub c_size: u64,
    pub data_off: u64,
    /// BLAKE3 hash of the uncompressed chunk data.
    /// All-zeros for entries read from v3 archives (no hash was stored).
    pub blake3: [u8; 32],
}

/// Serialize a chunk table in v4 format (64 bytes per entry).
pub fn write_table(mut w: impl Write, entries: &[ChunkEntry]) -> Result<()> {
    let mut buf = [0u8; ENTRY_SIZE];
    for e in entries {
        buf[0] = e.codec;
        for b in &mut buf[1..8] { *b = 0; } // padding
        buf[8..16].copy_from_slice(&e.u_size.to_le_bytes());
        buf[16..24].copy_from_slice(&e.c_size.to_le_bytes());
        buf[24..32].copy_from_slice(&e.data_off.to_le_bytes());
        buf[32..64].copy_from_slice(&e.blake3);
        w.write_all(&buf)?;
    }
    Ok(())
}

#[inline]
fn le64(x: &[u8]) -> u64 {
    u64::from_le_bytes(x.try_into().unwrap())
}

/// Parse a chunk table from a byte slice. Auto-detects v4 (64B) vs v3 (32B) by buffer length.
pub fn read_table_from_slice(buf: &[u8], count: u64) -> io::Result<Vec<ChunkEntry>> {
    if count == 0 {
        return Ok(Vec::new());
    }
    let expected_v4 = count as usize * ENTRY_SIZE;
    let expected_v3 = count as usize * ENTRY_SIZE_V3;

    let entry_size = if buf.len() == expected_v4 {
        ENTRY_SIZE
    } else if buf.len() == expected_v3 {
        ENTRY_SIZE_V3
    } else {
        return Err(io::Error::new(
            io::ErrorKind::UnexpectedEof,
            format!(
                "chunk table size mismatch: got {} bytes for {} chunks \
                 (expected {} for v4 or {} for v3)",
                buf.len(), count, expected_v4, expected_v3
            ),
        ));
    };

    let mut out = Vec::with_capacity(count as usize);
    let mut off = 0usize;
    for _ in 0..count {
        let e = &buf[off..off + entry_size];
        let codec = e[0];
        let u_size = le64(&e[8..16]);
        let c_size = le64(&e[16..24]);
        let data_off = le64(&e[24..32]);
        let blake3 = if entry_size == ENTRY_SIZE {
            e[32..64].try_into().unwrap()
        } else {
            [0u8; 32]
        };
        out.push(ChunkEntry { codec, u_size, c_size, data_off, blake3 });
        off += entry_size;
    }
    Ok(out)
}

/// Parse a chunk table from a `&[u8]` cursor, advancing past consumed bytes.
pub fn read_table(r: &mut &[u8], count: u64) -> Result<Vec<ChunkEntry>> {
    let dbg = env::var_os("ARX_DEBUG_LIST").is_some();

    if count == 0 {
        return Ok(Vec::new());
    }

    let entry_size = if r.len() >= count as usize * ENTRY_SIZE {
        ENTRY_SIZE
    } else if r.len() >= count as usize * ENTRY_SIZE_V3 {
        ENTRY_SIZE_V3
    } else {
        if dbg {
            eprintln!(
                "[DBG] chunktab: insufficient bytes: have={}, need at least {} for {} entries",
                r.len(), count as usize * ENTRY_SIZE_V3, count
            );
        }
        return Err(
            io::Error::new(io::ErrorKind::UnexpectedEof, "chunk table too small").into(),
        );
    };

    let need = count as usize * entry_size;
    let slice = &r[..need];
    let mut out = Vec::with_capacity(count as usize);
    let mut off = 0usize;

    for i in 0..(count as usize) {
        let codec = slice[off];
        let u_size = le64(&slice[off + 8..off + 16]);
        let c_size = le64(&slice[off + 16..off + 24]);
        let data_off = le64(&slice[off + 24..off + 32]);
        let blake3 = if entry_size == ENTRY_SIZE {
            slice[off + 32..off + 64].try_into().unwrap()
        } else {
            [0u8; 32]
        };

        if dbg {
            eprintln!("[DBG] CE[{i}]: codec={codec} u={u_size} c={c_size} off={data_off}");
        }

        out.push(ChunkEntry { codec, u_size, c_size, data_off, blake3 });
        off += entry_size;
    }

    *r = &r[need..];
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Vec<ChunkEntry> {
        vec![
            ChunkEntry { codec: 0, u_size: 65536, c_size: 65536, data_off: 1000, blake3: [0x11; 32] },
            ChunkEntry { codec: 1, u_size: 131072, c_size: 98304, data_off: 66536, blake3: [0x22; 32] },
        ]
    }

    #[test]
    fn test_v4_roundtrip() {
        let entries = sample();
        let mut buf = Vec::new();
        write_table(&mut buf, &entries).unwrap();
        assert_eq!(buf.len(), entries.len() * ENTRY_SIZE, "each v4 entry is 64 bytes");

        let back = read_table_from_slice(&buf, 2).unwrap();
        assert_eq!(back[0].codec, 0);
        assert_eq!(back[0].u_size, 65536);
        assert_eq!(back[0].blake3, [0x11; 32]);
        assert_eq!(back[1].codec, 1);
        assert_eq!(back[1].blake3, [0x22; 32]);
    }

    #[test]
    fn test_v3_compat_zeros_blake3() {
        // Build a v3-format table (32 bytes/entry, no hash)
        let mut buf = Vec::new();
        let mut entry = [0u8; ENTRY_SIZE_V3];
        entry[0] = 1; // Zstd
        entry[8..16].copy_from_slice(&200u64.to_le_bytes());
        entry[16..24].copy_from_slice(&150u64.to_le_bytes());
        entry[24..32].copy_from_slice(&9000u64.to_le_bytes());
        buf.extend_from_slice(&entry);

        let back = read_table_from_slice(&buf, 1).unwrap();
        assert_eq!(back[0].codec, 1);
        assert_eq!(back[0].u_size, 200);
        assert_eq!(back[0].blake3, [0u8; 32], "v3 entries should have zero blake3");
    }

    #[test]
    fn test_read_table_cursor_advances() {
        let entries = sample();
        let mut buf = Vec::new();
        write_table(&mut buf, &entries).unwrap();
        let mut slice = buf.as_slice();
        let back = read_table(&mut slice, 2).unwrap();
        assert_eq!(back.len(), 2);
        assert!(slice.is_empty(), "cursor should be fully consumed");
    }
}
