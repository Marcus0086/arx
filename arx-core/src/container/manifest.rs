use serde::{Deserialize, Serialize};
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ChunkRef {
    pub id: u64,     // index into ChunkTable
    pub u_size: u64, // uncompressed size of this chunk
}
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct FileEntry {
    pub path: String,
    pub mode: u32,
    pub mtime: i64,
    pub u_size: u64,
    pub chunk_refs: Vec<ChunkRef>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct DirEntry {
    pub path: String,
    pub mode: u32,
    pub mtime: i64,
}

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct Meta {
    pub created: i64,
    pub tool: String,
}

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct Manifest {
    pub files: Vec<FileEntry>,
    pub dirs: Vec<DirEntry>,
    pub meta: Meta,
}
