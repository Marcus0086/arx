use crate::error::Result;
use serde::{Deserialize, Serialize};
use std::io::{Read, Write};

#[repr(u8)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
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

pub fn get_decoder_u8(codec: u8) -> Result<&'static dyn Compressor> {
    match codec {
        val if val == CodecId::Store as u8 => Ok(&store::Store),
        val if val == CodecId::Zstd as u8 => Ok(&zstdc::ZstdCompressor),
        _ => Err(std::io::Error::new(std::io::ErrorKind::Other, "unknown codec id").into()),
    }
}
