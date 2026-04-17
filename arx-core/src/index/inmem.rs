use std::collections::{BTreeMap, HashMap};

use crate::codec::CodecId;
use crate::container::journal::{ChunkRef, Loc, LogRecord};
use crate::error::Result;
use crate::policy::Policy;
use crate::read::opened::Opened;
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
    /// Build an index from a base archive's manifest + chunk table.
    /// Each file entry is recorded with `Loc::Base` chunks pointing at the data
    /// region of the base `.arx` file.
    pub fn from_base(opened: &Opened) -> Result<Self> {
        let mut idx = InMemIndex::default();

        for fe in &opened.manifest.files {
            let chunks: Vec<ChunkRef> = fe
                .chunk_refs
                .iter()
                .map(|cr| {
                    let ce = &opened.table[cr.id as usize];
                    let codec = match ce.codec {
                        0 => CodecId::Store,
                        _ => CodecId::Zstd,
                    };
                    ChunkRef {
                        loc: Loc::Base,
                        // For Base chunks, off stores the chunk TABLE index so we
                        // can reconstruct the data_off and perform AEAD with the
                        // right nonce (nonce is derived from chunk_id).
                        off: cr.id,
                        len: ce.c_size,
                        codec,
                        blake3: ce.blake3,
                    }
                })
                .collect();

            idx.by_path.insert(
                fe.path.clone(),
                Entry {
                    mode: fe.mode,
                    mtime: fe.mtime as u64,
                    size: fe.u_size,
                    chunks,
                },
            );
        }

        idx.stats.files = opened.manifest.files.len() as u64;
        idx.stats.dirs = opened.manifest.dirs.len() as u64;
        idx.stats.chunks = opened.table.len() as u64;
        idx.stats.logical_bytes = opened.manifest.files.iter().map(|f| f.u_size).sum();

        Ok(idx)
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
                let e = Entry {
                    mode: *mode,
                    mtime: *mtime,
                    size: *size,
                    chunks: chunks.clone(),
                };
                self.by_path.insert(path.clone(), e);
                for c in chunks {
                    self.by_chunk
                        .insert(c.blake3, (c.loc, c.off, c.len, c.codec));
                }
                self.stats.files = self.stats.files.saturating_add(1);
                self.stats.logical_bytes = self.stats.logical_bytes.saturating_add(*size);
            }
            LogRecord::Delete { path } => {
                if let Some(e) = self.by_path.remove(path) {
                    self.stats.files = self.stats.files.saturating_sub(1);
                    self.stats.logical_bytes = self.stats.logical_bytes.saturating_sub(e.size);
                }
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
