use std::io::{Read, Write};

// ! ARX Superblock Layout
// !
// ! Version 4 (80 bytes, little-endian):
// !   0-5   (6B)  Magic "ARXALP"
// !   6-7   (2B)  Version: u16
// !   8-15  (8B)  Manifest length: u64
// !   16-23 (8B)  Chunk table offset: u64
// !   24-31 (8B)  Chunk count: u64
// !   32-39 (8B)  Data offset: u64
// !   40-47 (8B)  Flags: u64
// !   48-79 (32B) KDF salt: [u8; 32]   ← added in v4
// !
// ! Version 3 (48 bytes): same layout without kdf_salt.
// ! v3 archives are readable; all new archives are written as v4.
// !
// ! Flags:
// !   Bit 0: FLAG_ENCRYPTED     — archive uses per-region AEAD encryption
// !   Bit 1: FLAG_KDF_PASSWORD  — key was derived via Argon2id from a password

pub const MAGIC: &[u8; 6] = b"ARXALP";
pub const VERSION: u16 = 4;

/// Header length for v4+ archives.
pub const HEADER_LEN: u64 = 80; // 6+2+8+8+8+8+8+32
/// Header length for v3 archives (backward compatibility).
pub const HEADER_LEN_V3: u64 = 48;

pub const FLAG_ENCRYPTED: u64 = 1 << 0;
pub const FLAG_KDF_PASSWORD: u64 = 1 << 1;

#[derive(Debug, Clone, Copy)]
pub struct Superblock {
    pub version: u16,
    /// Byte length of the manifest region (includes AEAD tag when encrypted).
    pub manifest_len: u64,
    pub chunk_table_off: u64,
    pub chunk_count: u64,
    /// Absolute file offset where the data section starts.
    pub data_off: u64,
    pub flags: u64,
    /// Per-archive random salt used for nonce derivation and password KDF.
    /// All-zeros for v3 archives (no KDF salt in older format).
    pub kdf_salt: [u8; 32],
}

impl Superblock {
    /// The actual header length based on this archive's version.
    pub fn header_len(&self) -> u64 {
        if self.version >= 4 { HEADER_LEN } else { HEADER_LEN_V3 }
    }

    pub fn write_to(&self, mut w: impl Write) -> std::io::Result<()> {
        w.write_all(MAGIC)?;
        w.write_all(&self.version.to_le_bytes())?;
        w.write_all(&self.manifest_len.to_le_bytes())?;
        w.write_all(&self.chunk_table_off.to_le_bytes())?;
        w.write_all(&self.chunk_count.to_le_bytes())?;
        w.write_all(&self.data_off.to_le_bytes())?;
        w.write_all(&self.flags.to_le_bytes())?;
        w.write_all(&self.kdf_salt)?;
        Ok(())
    }

    pub fn read_from(mut r: impl Read) -> std::io::Result<Self> {
        let mut magic = [0u8; 6];
        r.read_exact(&mut magic)?;
        if &magic != MAGIC {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("bad archive magic: {:?}", &magic),
            ));
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

        let mut flags_buf = [0u8; 8];
        r.read_exact(&mut flags_buf)?;
        let flags = u64::from_le_bytes(flags_buf);

        // v4+ stores a 32-byte KDF salt; v3 does not — default to all-zeros
        let kdf_salt = if version >= 4 {
            let mut salt = [0u8; 32];
            r.read_exact(&mut salt)?;
            salt
        } else {
            [0u8; 32]
        };

        Ok(Self { version, manifest_len, chunk_table_off, chunk_count, data_off, flags, kdf_salt })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    fn sample_v4() -> Superblock {
        Superblock {
            version: VERSION,
            manifest_len: 512,
            chunk_table_off: 600,
            chunk_count: 3,
            data_off: 696,
            flags: FLAG_ENCRYPTED | FLAG_KDF_PASSWORD,
            kdf_salt: [0xABu8; 32],
        }
    }

    #[test]
    fn test_v4_roundtrip() {
        let sb = sample_v4();
        let mut buf = Vec::new();
        sb.write_to(&mut buf).unwrap();
        assert_eq!(buf.len(), HEADER_LEN as usize, "v4 header must be 80 bytes");

        let sb2 = Superblock::read_from(Cursor::new(&buf)).unwrap();
        assert_eq!(sb2.version, VERSION);
        assert_eq!(sb2.manifest_len, 512);
        assert_eq!(sb2.chunk_table_off, 600);
        assert_eq!(sb2.chunk_count, 3);
        assert_eq!(sb2.data_off, 696);
        assert_eq!(sb2.flags, FLAG_ENCRYPTED | FLAG_KDF_PASSWORD);
        assert_eq!(sb2.kdf_salt, [0xABu8; 32]);
    }

    #[test]
    fn test_v3_backward_compat() {
        // Craft a hand-built v3 header (48 bytes)
        let mut buf = Vec::new();
        buf.extend_from_slice(MAGIC);
        buf.extend_from_slice(&3u16.to_le_bytes());   // version 3
        buf.extend_from_slice(&100u64.to_le_bytes()); // manifest_len
        buf.extend_from_slice(&200u64.to_le_bytes()); // chunk_table_off
        buf.extend_from_slice(&2u64.to_le_bytes());   // chunk_count
        buf.extend_from_slice(&264u64.to_le_bytes()); // data_off
        buf.extend_from_slice(&0u64.to_le_bytes());   // flags
        assert_eq!(buf.len(), HEADER_LEN_V3 as usize);

        let sb = Superblock::read_from(Cursor::new(&buf)).unwrap();
        assert_eq!(sb.version, 3);
        assert_eq!(sb.kdf_salt, [0u8; 32], "v3 kdf_salt should be all-zeros");
        assert_eq!(sb.header_len(), HEADER_LEN_V3);
    }

    #[test]
    fn test_bad_magic_rejected() {
        let mut buf = b"BADMAG".to_vec();
        buf.extend_from_slice(&[0u8; 74]);
        let err = Superblock::read_from(Cursor::new(&buf)).unwrap_err();
        assert_eq!(err.kind(), std::io::ErrorKind::InvalidData);
    }
}
