use super::opened::Opened;
use crate::crypto::aead::{Region, derive_nonce};
use crate::error::Result;
use std::io::{Cursor, Read, Seek, SeekFrom};

pub struct FileReader<'a> {
    arx: &'a Opened,
    chunk_ids: Vec<u32>,
    cur: usize,
    cur_buf: Option<Cursor<Vec<u8>>>,
}

impl<'a> FileReader<'a> {
    pub fn new(arx: &'a Opened, path: &str) -> Result<Self> {
        let map = arx.chunk_map_for(path)?;
        Ok(Self {
            arx,
            chunk_ids: map.into_iter().map(|v| v.id as u32).collect(),
            cur: 0,
            cur_buf: None,
        })
    }

    fn load_next(&mut self) -> std::io::Result<bool> {
        if self.cur >= self.chunk_ids.len() {
            return Ok(false);
        }
        let idx = self.chunk_ids[self.cur] as usize;
        let ce = &self.arx.table[idx];

        // read ciphertext
        let mut f = self
            .arx
            .f
            .lock()
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?;
        f.seek(SeekFrom::Start(ce.data_off))?;
        let mut ct = vec![0u8; ce.c_size as usize];
        f.read_exact(&mut ct)?;
        drop(f);

        // AEAD open (if enabled) — uses Data region with chunk index as counter
        let pt = if let Some((ref key, salt)) = self.arx.aead {
            let nonce = derive_nonce(&salt, Region::ChunkData, idx as u64);
            crate::crypto::aead::open_whole(key, &nonce, b"chunk", &ct)
        } else {
            ct
        };

        // decompress (Store/Zstd)
        let mut plain = vec![0u8; ce.u_size as usize];
        let n = crate::codec::get_decoder_u8(ce.codec as u8)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?
            .decompress(&mut pt.as_slice(), &mut plain.as_mut_slice())
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?;
        plain.truncate(n as usize);

        self.cur += 1;
        self.cur_buf = Some(Cursor::new(plain));
        Ok(true)
    }
}

impl<'a> Read for FileReader<'a> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        loop {
            if let Some(ref mut cur) = self.cur_buf {
                let n = cur.read(buf)?;
                if n > 0 {
                    return Ok(n);
                }
                self.cur_buf = None;
            }
            if !self.load_next()? {
                return Ok(0);
            }
        }
    }
}

pub struct RangeReader<'a> {
    inner: FileReader<'a>,
    remain: u64,
}

impl<'a> RangeReader<'a> {
    pub fn new(arx: &'a Opened, path: &str, start: u64, len: u64) -> Result<Self> {
        let mut fr = FileReader::new(arx, path)?;
        // advance by consuming `start` bytes (bounded: per-chunk buffer only)
        std::io::copy(&mut (&mut fr).take(start), &mut std::io::sink())?;
        Ok(Self {
            inner: fr,
            remain: len,
        })
    }
}

impl<'a> Read for RangeReader<'a> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        if self.remain == 0 {
            return Ok(0);
        }
        let cap = std::cmp::min(self.remain, buf.len() as u64) as usize;
        let n = (&mut self.inner).read(&mut buf[..cap])?;
        self.remain -= n as u64;
        Ok(n)
    }
}
