use crate::error::Result;
use std::io::{Read, Write};

#[repr(u8)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum CodecId {
    Store = 0,
    Zstd = 1,
}

pub trait Compressor: Send + Sync {
    fn id(&self) -> CodecId;
    fn compress(&self, src: &mut dyn Read, dst: &mut dyn Write, level: i32) -> Result<u64>;
    fn decompress(&self, src: &mut dyn Read, dst: &mut dyn Write) -> Result<u64>;
}

pub mod store;
pub mod zstdc;
