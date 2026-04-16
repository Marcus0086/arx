use std::io::{self, Read, Write};

/// Encode a u64 as a little-endian base-128 varint into any `Write`.
pub fn write_uvarint(out: &mut impl Write, mut x: u64) -> io::Result<()> {
    while x >= 0x80 {
        out.write_all(&[(x as u8) | 0x80])?;
        x >>= 7;
    }
    out.write_all(&[x as u8])
}

/// Decode a varint from any `Read`. Returns `None` at clean EOF (zero bytes read).
pub fn read_uvarint<R: Read>(r: &mut R) -> io::Result<Option<u64>> {
    let mut x: u64 = 0;
    let mut s: u32 = 0;
    for _ in 0..10 {
        let mut b = [0u8; 1];
        match r.read(&mut b) {
            Ok(0) => return Ok(None),
            Ok(_) => {
                let byte = b[0];
                if byte < 0x80 {
                    x |= (byte as u64) << s;
                    return Ok(Some(x));
                }
                x |= ((byte & 0x7f) as u64) << s;
                s += 7;
            }
            Err(e) => return Err(e),
        }
    }
    Err(io::Error::new(io::ErrorKind::InvalidData, "varint too long"))
}

/// Number of bytes the varint encoding of `x` occupies.
pub fn uvarint_len(mut x: u64) -> usize {
    let mut n = 1;
    while x >= 0x80 {
        x >>= 7;
        n += 1;
    }
    n
}

#[cfg(test)]
mod tests {
    use super::*;

    fn roundtrip(v: u64) {
        let mut buf = Vec::new();
        write_uvarint(&mut buf, v).unwrap();
        assert_eq!(buf.len(), uvarint_len(v), "len mismatch for {v}");
        let result = read_uvarint(&mut buf.as_slice()).unwrap();
        assert_eq!(result, Some(v), "value mismatch for {v}");
    }

    #[test]
    fn test_roundtrip_boundary_values() {
        for v in [0u64, 1, 127, 128, 255, 300, 16383, 16384, u32::MAX as u64, u64::MAX] {
            roundtrip(v);
        }
    }

    #[test]
    fn test_clean_eof_returns_none() {
        let buf: &[u8] = &[];
        assert!(read_uvarint(&mut buf.as_ref()).unwrap().is_none());
    }
}
