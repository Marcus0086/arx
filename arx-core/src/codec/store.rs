use super::{CodecId, Compressor};
use crate::error::Result;
use std::io::{Read, Write};

pub struct Store;

impl Compressor for Store {
    fn id(&self) -> CodecId {
        CodecId::Store
    }

    fn compress(&self, src: &mut dyn Read, dst: &mut dyn Write, _level: i32) -> Result<u64> {
        Ok(std::io::copy(src, dst)?)
    }

    fn decompress(&self, src: &mut dyn Read, dst: &mut dyn Write) -> Result<u64> {
        Ok(std::io::copy(src, dst)?)
    }
}
