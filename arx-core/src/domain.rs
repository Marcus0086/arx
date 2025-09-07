// arx_core/src/domain.rs
#[derive(Clone, Debug)]
pub struct FileRow {
    pub path: String,
    pub u_size: u64,
    pub chunks: usize,
    pub encrypted: bool,
}

#[derive(Clone, Debug)]
pub struct ChunkRow {
    pub ordinal: u64,
    pub id: u64,
    pub codec: u8,
    pub file_off: u64,
    pub u_len: u64,
    pub c_len: u64,
    pub data_off: u64,
    pub pct_end: f32,
}
