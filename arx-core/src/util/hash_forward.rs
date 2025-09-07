use std::io::{Result, Write};

pub struct HashingForward<'a, W: Write> {
    inner: W,
    hasher: &'a mut blake3::Hasher,
    // Optionally track total compressed bytes that passed through
    pub counted_c: u64,
}

impl<'a, W: Write> HashingForward<'a, W> {
    pub fn new(inner: W, hasher: &'a mut blake3::Hasher) -> Self {
        Self {
            inner,
            hasher,
            counted_c: 0,
        }
    }
    pub fn into_inner(self) -> W {
        self.inner
    }
}

impl<'a, W: Write> Write for HashingForward<'a, W> {
    fn write(&mut self, buf: &[u8]) -> Result<usize> {
        self.hasher.update(buf);
        self.counted_c += buf.len() as u64;
        self.inner.write(buf)
    }
    fn flush(&mut self) -> Result<()> {
        self.inner.flush()
    }
}
