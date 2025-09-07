use std::io::Read;
use std::sync::Arc;

use crate::domain::{ChunkRow, FileRow};
use crate::error::Result;
use crate::read::opened::Opened;
use crate::repo::{ArchiveRepo, OpenParams};

pub struct FsArchiveRepo {
    opened: Arc<Opened>,
}

impl FsArchiveRepo {
    pub fn new(params: OpenParams) -> Result<Self> {
        let opened = Opened::open(&params.archive_path, params.aead_key, params.key_salt)?;
        Ok(Self {
            opened: Arc::new(opened),
        })
    }
}

impl ArchiveRepo for FsArchiveRepo {
    fn list_files(&self) -> Result<Vec<FileRow>> {
        let enc = self.opened.aead.is_some();
        let rows = self
            .opened
            .list_entries()
            .map(|e| FileRow {
                path: e.path,
                u_size: e.u_size,
                chunks: e.chunks.len(),
                encrypted: enc,
            })
            .collect();
        Ok(rows)
    }

    fn chunk_map(&self, path: &str) -> Result<Vec<ChunkRow>> {
        let v = self.opened.chunk_map_for(path)?;
        Ok(v.into_iter()
            .map(|r| ChunkRow {
                ordinal: r.ordinal,
                id: r.id,
                codec: r.codec,
                file_off: r.file_off,
                u_len: r.u_len,
                c_len: r.c_len,
                data_off: r.data_off,
                pct_end: r.pct_end,
            })
            .collect())
    }

    fn open_reader(&self, path: &str) -> Result<Box<dyn Read + Send + '_>> {
        let r = self.opened.open_reader(path)?;
        // Box to erase type; keep it Send
        Ok(Box::new(r))
    }

    fn open_range(&self, path: &str, start: u64, len: u64) -> Result<Box<dyn Read + Send + '_>> {
        let r = self.opened.open_range(path, start, len)?;
        Ok(Box::new(r))
    }
}
