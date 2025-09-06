use serde::{Deserialize, Serialize};
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct FileEntry {
    pub path: String,
    pub mode: u32,
    pub mtime: i64,
    pub size: u64,
    pub data_off: u64, // absolute offset in archive where file bytes begin
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
