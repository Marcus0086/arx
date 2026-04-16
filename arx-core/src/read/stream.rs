use super::opened::Opened;
use crate::crypto::aead::{Region, derive_nonce};
use crate::error::Result;
use std::io::{Cursor, Read};

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

        // Lock-free positional read (no Mutex needed)
        let ct = self.arx.read_chunk_bytes(ce.data_off, ce.c_size)?;

        // AEAD decrypt if enabled
        let pt = if let Some((ref key, salt)) = self.arx.aead {
            let nonce = derive_nonce(&salt, Region::ChunkData, idx as u64);
            crate::crypto::aead::open_whole(key, &nonce, b"chunk", &ct)
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string()))?
        } else {
            ct
        };

        // Decompress
        let mut plain = Vec::with_capacity(ce.u_size as usize);
        crate::codec::get_decoder_u8(ce.codec)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?
            .decompress(&mut pt.as_slice(), &mut plain)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?;

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
        let map = arx.chunk_map_for(path)?;

        // Find which chunk index `start` falls in and the byte offset within that chunk
        let mut chunk_start_idx = 0usize;
        let mut offset_in_chunk = start;
        let chunk_ids: Vec<u32> = map.iter().map(|v| v.id as u32).collect();

        for (i, cv) in map.iter().enumerate() {
            if offset_in_chunk < cv.u_len {
                chunk_start_idx = i;
                break;
            }
            offset_in_chunk -= cv.u_len;
        }

        // Build a FileReader starting at the right chunk
        let mut fr = FileReader {
            arx,
            chunk_ids,
            cur: chunk_start_idx,
            cur_buf: None,
        };

        // Consume intra-chunk offset bytes
        if offset_in_chunk > 0 {
            std::io::copy(&mut (&mut fr).take(offset_in_chunk), &mut std::io::sink())?;
        }

        Ok(Self { inner: fr, remain: len })
    }
}

impl<'a> Read for RangeReader<'a> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        if self.remain == 0 {
            return Ok(0);
        }
        let cap = std::cmp::min(self.remain, buf.len() as u64) as usize;
        let n = self.inner.read(&mut buf[..cap])?;
        self.remain -= n as u64;
        Ok(n)
    }
}
