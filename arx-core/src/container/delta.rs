use std::fs::{File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

use crate::container::journal::EncMode;
use crate::error::Result;

use chacha20poly1305::aead::{Aead, KeyInit};
use chacha20poly1305::{XChaCha20Poly1305, XNonce};

pub struct DeltaStore {
    f: File,
    pub path: PathBuf,
    pub next_off: u64,
    enc: EncMode,
    salt: [u8; 32],
}

fn put_uvarint(out: &mut Vec<u8>, mut x: u64) {
    while x >= 0x80 {
        out.push((x as u8) | 0x80);
        x >>= 7;
    }
    out.push(x as u8);
}

fn uvarint_len(mut x: u64) -> usize {
    let mut n = 1;
    while x >= 0x80 {
        x >>= 7;
        n += 1;
    }
    n
}

impl DeltaStore {
    pub fn open(path: &Path, enc: EncMode) -> Result<Self> {
        let existed = path.exists();
        let mut f = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(path)?;
        let next_off = if existed {
            f.seek(SeekFrom::End(0))?
        } else {
            0
        };
        let salt = match enc {
            EncMode::Plain => [0u8; 32],
            EncMode::Aead { salt, .. } => salt,
        };
        Ok(Self {
            f,
            path: path.to_path_buf(),
            next_off,
            enc,
            salt,
        })
    }

    pub fn append_frame(&mut self, frame_plain: &[u8]) -> Result<(u64, u64)> {
        match self.enc {
            EncMode::Plain => {
                let off_before = self.f.stream_position()?;
                let mut lenv = Vec::with_capacity(10);
                put_uvarint(&mut lenv, frame_plain.len() as u64);
                self.f.write_all(&lenv)?;
                self.f.write_all(frame_plain)?;
                self.f.flush()?;
                let payload_off = off_before + lenv.len() as u64;
                self.next_off = payload_off + frame_plain.len() as u64;
                Ok((payload_off, frame_plain.len() as u64))
            }
            EncMode::Aead { key, .. } => {
                let pos = self.f.stream_position()?;
                let cipher_len = (frame_plain.len() as u64) + 16;
                let varint_len = uvarint_len(cipher_len);
                let payload_off = pos + varint_len as u64;

                let mut hasher = blake3::Hasher::new();
                hasher.update(b"arxdelta");
                hasher.update(&self.salt);
                hasher.update(&payload_off.to_le_bytes());
                hasher.update(&cipher_len.to_le_bytes());
                let hb = hasher.finalize();
                let mut nonce = [0u8; 24];
                nonce.copy_from_slice(&hb.as_bytes()[..24]);

                let cipher = XChaCha20Poly1305::new((&key).into());
                let ct = cipher
                    .encrypt(&XNonce::from(nonce), frame_plain)
                    .map_err(|_| std::io::Error::new(std::io::ErrorKind::Other, "aead encrypt"))?;

                let mut lenv = Vec::with_capacity(10);
                put_uvarint(&mut lenv, ct.len() as u64);
                self.f.write_all(&lenv)?;
                self.f.write_all(&ct)?;
                self.f.flush()?;
                self.next_off = payload_off + ct.len() as u64;
                Ok((payload_off, ct.len() as u64))
            }
        }
    }

    pub fn read_frame(&self, off: u64, len: u64) -> Result<Box<dyn Read + Send>> {
        let mut f = self.f.try_clone()?;
        f.seek(SeekFrom::Start(off))?;
        let mut buf = vec![0u8; len as usize];
        f.read_exact(&mut buf)?;

        let plain = match self.enc {
            EncMode::Plain => buf,
            EncMode::Aead { key, .. } => {
                let mut hasher = blake3::Hasher::new();
                hasher.update(b"arxdelta");
                hasher.update(&self.salt);
                hasher.update(&off.to_le_bytes());
                hasher.update(&len.to_le_bytes());
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

        Ok(Box::new(std::io::Cursor::new(plain)))
    }
}
