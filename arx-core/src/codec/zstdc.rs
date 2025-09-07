use super::{CodecId, Compressor};
use crate::error::Result;
use std::io::{Read, Write};

pub struct ZstdCompressor;

impl Compressor for ZstdCompressor {
    fn id(&self) -> CodecId {
        CodecId::Zstd
    }
    fn compress(&self, src: &mut dyn Read, dst: &mut dyn Write, level: i32) -> Result<u64> {
        let mut enc = zstd::stream::Encoder::new(dst, level.max(1))?;
        // Optional: enable multithreading when compiled with the "zstdmt" feature.
        #[cfg(feature = "zstdmt")]
        {
            let _ = enc.multithread(0);
        }
        let mut w = enc.auto_finish();
        let written_uncompressed = std::io::copy(src, &mut w)?;
        Ok(written_uncompressed)
    }

    fn decompress(&self, src: &mut dyn Read, dst: &mut dyn Write) -> Result<u64> {
        let mut dec = zstd::stream::Decoder::new(src)?;
        let written_uncompressed = std::io::copy(&mut dec, dst)?;
        Ok(written_uncompressed)
    }
}
