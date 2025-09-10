use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Stats {
    pub files: u64,
    pub dirs: u64,
    pub chunks: u64,
    pub logical_bytes: u64,
    pub physical_bytes_base: u64,
    pub physical_bytes_delta: u64,
    pub compression_ratio: f32,
    pub last_commit_ts: u64,
}
