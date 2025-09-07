#![forbid(unsafe_code)]

pub mod error;
pub mod policy;

pub mod util {
    pub mod buf;
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

// Re-exports: stable API surface
pub use list::list;
pub use pack::writer::{Encryption, PackOptions, pack};
pub use read::extract::extract;
