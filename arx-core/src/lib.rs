#![forbid(unsafe_code)]

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

pub mod error;
pub mod policy;

pub mod util {
    pub mod buf;
    pub mod hash_forward;
    pub mod sanitize;
    pub mod varint;
}

pub mod chunking {
    pub mod fastcdc;
}

pub mod hash {
    pub mod blake3;
}

pub mod codec;

pub mod crypto {
    pub mod aead;
    pub mod hex;
    pub mod kdf;
    pub mod nonce;
}

pub mod container {
    pub mod chunktab;
    pub mod manifest;
    pub mod superblock;
    pub mod tail;
}

pub mod pack {
    pub mod walker;
    pub mod writer;
}

pub mod read {
    pub mod extract;
    pub mod reader;
}

pub mod list;

pub use crate::error::Result;

pub use pack::writer::{PackOptions, pack};

pub use read::extract::{ExtractOptions, extract};

pub use list::{ListOptions, list};

pub use container::chunktab::ChunkEntry;
pub use container::manifest::{DirEntry, FileEntry, Manifest};
pub use container::superblock::Superblock;

pub mod prelude {
    pub use crate::Result;
    pub use crate::codec::CodecId;
    pub use crate::container::manifest::{DirEntry, FileEntry, Manifest};
    pub use crate::list::{ListOptions, list};
    pub use crate::pack::writer::{PackOptions, pack};
    pub use crate::read::extract::{ExtractOptions, extract};
}
