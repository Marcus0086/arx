use std::collections::{BTreeMap, HashMap};

use crate::codec::CodecId;
use crate::container::journal::{ChunkRef, Loc, LogRecord};
use crate::error::Result;
use crate::policy::Policy;
use crate::stats::Stats;

#[derive(Clone, Debug)]
pub struct Entry {
    pub mode: u32,
    pub mtime: u64,
    pub size: u64,
    pub chunks: Vec<ChunkRef>,
}

#[derive(Clone, Debug, Default)]
pub struct InMemIndex {
    pub by_path: BTreeMap<String, Entry>,
    pub by_chunk: HashMap<[u8; 32], (Loc, u64, u64, CodecId)>,
    pub policy: Policy,
    pub stats: Stats,
}

impl InMemIndex {
    /// Build from base archive (manifest + chunk table). Placeholder for now; we’ll wire actual readers next.
    pub fn from_base() -> Result<Self> {
        Ok(Self::default())
    }

    pub fn apply(&mut self, rec: &LogRecord) {
        match rec {
            LogRecord::Put {
                path,
                mode,
                mtime,
                size,
                chunks,
            } => {
                // update by_path
                let e = Entry {
                    mode: *mode,
                    mtime: *mtime,
                    size: *size,
                    chunks: chunks.clone(),
                };
                self.by_path.insert(path.clone(), e);
                // update by_chunk
                for c in chunks {
                    self.by_chunk
                        .insert(c.blake3, (c.loc, c.off, c.len, c.codec));
                }
                self.stats.files += 1; // simplistic; refine later
                self.stats.logical_bytes += *size as u64;
            }
            LogRecord::Delete { path } => {
                self.by_path.remove(path);
            }
            LogRecord::Rename { from, to } => {
                if let Some(e) = self.by_path.remove(from) {
                    self.by_path.insert(to.clone(), e);
                }
            }
            LogRecord::SetPolicy(p) => {
                self.policy = p.clone();
            }
            LogRecord::Note { .. } => {}
        }
    }
}
