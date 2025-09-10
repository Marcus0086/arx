use serde::{Deserialize, Serialize};
use std::fs::{File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

use crate::codec::CodecId;
use crate::error::Result;
use crate::policy::Policy;

use chacha20poly1305::aead::{Aead, KeyInit};
use chacha20poly1305::{XChaCha20Poly1305, XNonce};

const MAGIC: &[u8; 8] = b"ARXLOG\0\0";
const VERSION: u8 = 1;
const FLAG_AEAD: u8 = 0b0000_0001;

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
pub enum Loc {
    Base,
    Delta,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ChunkRef {
    pub loc: Loc,
    pub off: u64,
    pub len: u64,
    pub codec: CodecId,
    pub blake3: [u8; 32],
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum LogRecord {
    Put {
        path: String,
        mode: u32,
        mtime: u64,
        size: u64,
        chunks: Vec<ChunkRef>,
    },
    Delete {
        path: String,
    },
    Rename {
        from: String,
        to: String,
    },
    SetPolicy(Policy),
    Note {
        text: String,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EncMode {
    Plain,
    Aead { key: [u8; 32], salt: [u8; 32] },
}

pub struct Journal {
    f: File,
    path: PathBuf,
    enc: EncMode,
    flags: u8,
    salt: [u8; 32],
}

pub struct JournalIter<'a> {
    f: &'a mut File,
    enc: EncMode,
    salt: [u8; 32],
}

impl<'a> Iterator for JournalIter<'a> {
    type Item = Result<LogRecord>;
    fn next(&mut self) -> Option<Self::Item> {
        match read_next_record(self.f, self.enc, self.salt) {
            Ok(Some(r)) => Some(Ok(r)),
            Ok(None) => None,
            Err(e) => Some(Err(e)),
        }
    }
}

fn read_next_record(f: &mut File, enc: EncMode, salt: [u8; 32]) -> Result<Option<LogRecord>> {
    let start = f.stream_position()?;
    let len = match get_uvarint(f) {
        Ok(Some(n)) => n,
        Ok(None) => return Ok(None),
        Err(e) => return Err(e),
    };
    let payload_off = start + uvarint_len(len) as u64;

    let mut buf = vec![0u8; len as usize];
    if let Err(e) = f.read_exact(&mut buf) {
        if e.kind() == std::io::ErrorKind::UnexpectedEof {
            return Ok(None);
        }
        return Err(e.into());
    }

    let plain = match enc {
        EncMode::Plain => buf,
        EncMode::Aead { key, .. } => {
            let mut hasher = blake3::Hasher::new();
            hasher.update(b"arxlog");
            hasher.update(&salt);
            hasher.update(&payload_off.to_le_bytes());
            hasher.update(&len.to_le_bytes()); // ciphertext len
            let hb = hasher.finalize();
            let mut nonce = [0u8; 24];
            nonce.copy_from_slice(&hb.as_bytes()[..24]);

            let cipher = XChaCha20Poly1305::new((&key).into());
            cipher
                .decrypt(&XNonce::from(nonce), buf.as_ref())
                .map_err(|_| {
                    std::io::Error::new(std::io::ErrorKind::InvalidData, "aead decrypt failed")
                })?
        }
    };

    let rec: LogRecord = serde_cbor::from_slice(&plain)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?;
    Ok(Some(rec))
}

fn put_uvarint(out: &mut Vec<u8>, mut x: u64) {
    while x >= 0x80 {
        out.push((x as u8) | 0x80);
        x >>= 7;
    }
    out.push(x as u8);
}

fn get_uvarint<R: Read>(r: &mut R) -> Result<Option<u64>> {
    let mut x: u64 = 0;
    let mut s: u32 = 0;
    for _ in 0..10 {
        let mut b = [0u8; 1];
        match r.read(&mut b) {
            Ok(0) => return Ok(None),
            Ok(_) => {
                let byte = b[0];
                if byte < 0x80 {
                    x |= ((byte as u64) << s) as u64;
                    return Ok(Some(x));
                }
                x |= (((byte & 0x7f) as u64) << s) as u64;
                s += 7;
            }
            Err(e) => return Err(e.into()),
        }
    }
    Err(std::io::Error::new(std::io::ErrorKind::InvalidData, "varint too long").into())
}

fn uvarint_len(mut x: u64) -> usize {
    let mut n = 1;
    while x >= 0x80 {
        x >>= 7;
        n += 1;
    }
    n
}

impl Journal {
    pub fn open(path: &Path, enc: EncMode) -> Result<Self> {
        let existed = path.exists();
        let mut f = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(path)?;
        let (flags, salt) = if !existed {
            // Write header
            let (flags, salt) = match enc {
                EncMode::Plain => (0u8, [0u8; 32]),
                EncMode::Aead { salt, .. } => (FLAG_AEAD, salt),
            };
            f.write_all(MAGIC)?;
            f.write_all(&[VERSION])?;
            f.write_all(&[flags])?;
            f.write_all(&salt)?;
            f.flush()?;
            (flags, salt)
        } else {
            // Validate header, read flags+salt (tolerate legacy header with no flags/salt)
            let mut magic = [0u8; 8];
            f.read_exact(&mut magic)?;
            if &magic != MAGIC {
                // Re-init conservatively
                f.seek(SeekFrom::Start(0))?;
                f.set_len(0)?;
                let (flags, salt) = match enc {
                    EncMode::Plain => (0u8, [0u8; 32]),
                    EncMode::Aead { salt, .. } => (FLAG_AEAD, salt),
                };
                f.write_all(MAGIC)?;
                f.write_all(&[VERSION])?;
                f.write_all(&[flags])?;
                f.write_all(&salt)?;
                f.flush()?;
                (flags, salt)
            } else {
                let mut ver = [0u8; 1];
                f.read_exact(&mut ver)?;
                let _ = ver[0]; // reserved
                // Try read flags+salt; if EOF (legacy), assume Plain
                let mut flags = [0u8; 1];
                let mut salt = [0u8; 32];
                match f.read_exact(&mut flags) {
                    Ok(_) => {
                        f.read_exact(&mut salt)?;
                        (flags[0], salt)
                    }
                    Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => (0, [0u8; 32]),
                    Err(e) => return Err(e.into()),
                }
            }
        };

        // Sanity: if file says AEAD but caller passed Plain, refuse (avoid gibberish reads)
        if flags & FLAG_AEAD != 0 {
            if let EncMode::Plain = enc {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::PermissionDenied,
                    "journal is AEAD-sealed; provide --key/--key-salt",
                )
                .into());
            }
        }

        // Seek to end for appends
        f.seek(SeekFrom::End(0))?;
        Ok(Self {
            f,
            path: path.to_path_buf(),
            enc,
            flags,
            salt,
        })
    }

    /// Append a single record (length-delimited). Partial tails are ignored on read.
    pub fn append(&mut self, rec: &LogRecord) -> Result<()> {
        let mut plain = Vec::with_capacity(256);
        serde_cbor::to_writer(&mut plain, rec)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?;

        match self.enc {
            EncMode::Plain => {
                let mut lenv = Vec::with_capacity(10);
                put_uvarint(&mut lenv, plain.len() as u64);
                self.f.write_all(&lenv)?;
                self.f.write_all(&plain)?;
                self.f.flush()?;
                Ok(())
            }
            EncMode::Aead { key, .. } => {
                // Compute payload_off deterministically
                let pos = self.f.stream_position()?;
                let cipher_len = (plain.len() as u64) + 16; // AEAD tag
                let varint_len = uvarint_len(cipher_len);
                let payload_off = pos + varint_len as u64;

                // Derive nonce from (payload_off, cipher_len)
                let mut hasher = blake3::Hasher::new();
                hasher.update(b"arxlog");
                hasher.update(&self.salt);
                hasher.update(&payload_off.to_le_bytes());
                hasher.update(&cipher_len.to_le_bytes());
                let hb = hasher.finalize();
                let mut nonce = [0u8; 24];
                nonce.copy_from_slice(&hb.as_bytes()[..24]);

                let cipher = XChaCha20Poly1305::new((&key).into());
                let ct = cipher
                    .encrypt(&XNonce::from(nonce), plain.as_ref())
                    .map_err(|_| std::io::Error::new(std::io::ErrorKind::Other, "aead encrypt"))?;

                // Write length (of ciphertext) + ciphertext
                let mut lenv = Vec::with_capacity(10);
                put_uvarint(&mut lenv, ct.len() as u64);
                self.f.write_all(&lenv)?;
                self.f.write_all(&ct)?;
                self.f.flush()?;
                Ok(())
            }
        }
    }
    /// Create an iterator starting after the header.
    pub fn iter(&mut self) -> Result<JournalIter<'_>> {
        self.f.flush()?;
        self.f
            .seek(SeekFrom::Start((MAGIC.len() + 1 + 1 + 32) as u64))?;
        Ok(JournalIter {
            f: &mut self.f,
            enc: self.enc,
            salt: self.salt,
        })
    }
}
