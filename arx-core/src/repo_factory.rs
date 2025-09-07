use crate::error::Result;
use crate::repo::{ArchiveRepo, OpenParams};
use crate::repo_fs::FsArchiveRepo;

pub enum Backend {
    Fs,
}

pub fn open_repo(backend: Backend, p: OpenParams) -> Result<Box<dyn ArchiveRepo>> {
    match backend {
        Backend::Fs => Ok(Box::new(FsArchiveRepo::new(p)?)),
    }
}
