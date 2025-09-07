// arx_core/src/repo.rs
use crate::domain::{ChunkRow, FileRow};
use crate::error::Result;
use std::io::Read;

#[derive(Clone, Debug)]
pub struct OpenParams {
    pub archive_path: std::path::PathBuf,
    pub aead_key: Option<[u8; 32]>,
    pub key_salt: [u8; 32],
}

pub trait ArchiveRepo: Send + Sync {
    fn list_files(&self) -> Result<Vec<FileRow>>;

    fn chunk_map(&self, path: &str) -> Result<Vec<ChunkRow>>;

    fn open_reader(&self, path: &str) -> Result<Box<dyn Read + Send + '_>>;

    fn open_range(&self, path: &str, start: u64, len: u64) -> Result<Box<dyn Read + Send + '_>>;
}
