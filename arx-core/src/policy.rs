use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Policy {
    pub max_entries: Option<u64>,
    pub max_uncompressed: Option<u64>,
    pub max_delta_bytes: Option<u64>,
    /// Reject puts that expand too much; e.g. 1.05 means require ≥1.05x compression.
    pub min_compression_ratio: Option<f32>,
    pub allow_symlinks: bool,
}
