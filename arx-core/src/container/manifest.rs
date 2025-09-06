use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct Manifest {
    pub files: Vec<FileEntry>,
    pub dirs: Vec<DirEntry>,
    pub meta: Meta,
}
