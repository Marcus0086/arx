use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(author, version, about = "arx — modern portable archive", long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}


#[derive(Subcommand)]
pub enum ChunkCommands {
    /// Print the chunk map for a file inside the archive.
    Chunks {
        archive: PathBuf,
        path: String,
        #[arg(long = "key")] key_hex: Option<String>,
        #[arg(long = "password")] password: Option<String>,
    },
    /// Stream a file (or byte range) to stdout.
    Cat {
        archive: PathBuf,
        path: String,
        #[arg(long, default_value_t = 0)] start: u64,
        #[arg(long)] len: Option<u64>,
        #[arg(long = "key")] key_hex: Option<String>,
        #[arg(long = "password")] password: Option<String>,
    },
    /// Download a file (or byte range) to a local path.
    Get {
        archive: PathBuf,
        path: String,
        out: PathBuf,
        #[arg(long, default_value_t = 0)] start: u64,
        #[arg(long)] len: Option<u64>,
        #[arg(long = "key")] key_hex: Option<String>,
        #[arg(long = "password")] password: Option<String>,
    },
}

#[derive(Subcommand)]
pub enum CrudCommands {
    /// Add a file (or directory with --recursive) into the overlay.
    Add {
        archive: PathBuf,
        /// Source file/directory on disk.
        src: PathBuf,
        /// Destination path prefix inside the archive (e.g. "/").
        dst: String,
        #[arg(long)] recursive: bool,
        /// File mode (octal). Inferred from src on Unix if omitted.
        #[arg(long)] mode: Option<u32>,
        /// mtime (seconds since epoch). Inferred from src metadata if omitted.
        #[arg(long)] mtime: Option<u64>,
        #[arg(long = "key")] key_hex: Option<String>,
        #[arg(long = "password")] password: Option<String>,
    },

    /// Delete (tombstone) a path from the overlay.
    Rm {
        archive: PathBuf,
        path: String,
        #[arg(long)] recursive: bool,
        #[arg(long = "key")] key_hex: Option<String>,
        #[arg(long = "password")] password: Option<String>,
    },

    /// Rename/move a path within the overlay.
    Mv {
        archive: PathBuf,
        from: String,
        to: String,
        #[arg(long = "key")] key_hex: Option<String>,
        #[arg(long = "password")] password: Option<String>,
    },

    /// List the merged overlay state (base + journal changes).
    Ls {
        archive: PathBuf,
        #[arg(long)] prefix: Option<String>,
        #[arg(long)] long: bool,
        #[arg(long = "key")] key_hex: Option<String>,
        #[arg(long = "password")] password: Option<String>,
    },

    /// Show what changed in the overlay vs the base archive.
    Diff {
        archive: PathBuf,
        #[arg(long = "key")] key_hex: Option<String>,
        #[arg(long = "password")] password: Option<String>,
    },

    /// Compact the overlay back into a fresh immutable base archive.
    /// If --out is omitted, overwrites the original archive in-place.
    Sync {
        archive: PathBuf,
        /// Output path for the compacted archive (defaults to in-place overwrite).
        #[arg(long)] out: Option<PathBuf>,
        #[arg(long)] deterministic: bool,
        #[arg(long, default_value_t = 0.05)] min_gain: f32,
        #[arg(long = "key")] key_hex: Option<String>,
        #[arg(long = "password")] password: Option<String>,
        /// Re-seal the compacted archive with the provided key.
        #[arg(long)] seal_base: bool,
    },

    /// Stream a file from the overlay to stdout.
    Cat {
        archive: PathBuf,
        path: String,
        #[arg(long = "key")] key_hex: Option<String>,
        #[arg(long = "password")] password: Option<String>,
    },

    /// Download a file from the overlay to a local path.
    Get {
        archive: PathBuf,
        path: String,
        out: PathBuf,
        #[arg(long = "key")] key_hex: Option<String>,
        #[arg(long = "password")] password: Option<String>,
    },
}

#[derive(Subcommand)]
pub enum Commands {
    /// Pack files/directories into an ARX archive.
    Pack {
        out: PathBuf,
        inputs: Vec<PathBuf>,
        #[arg(long)] deterministic: bool,
        #[arg(long, default_value_t = 0.05)] min_gain: f32,
        /// 32-byte hex key to enable AEAD encryption.
        #[arg(long = "encrypt-raw")] encrypt_raw_hex: Option<String>,
        /// Derive encryption key from a password (Argon2id).
        #[arg(long)] password: Option<String>,
    },

    /// List archive contents.
    List {
        archive: PathBuf,
        #[arg(long = "key")] key_hex: Option<String>,
        #[arg(long = "password")] password: Option<String>,
    },

    /// Extract archive to a destination directory.
    Extract {
        archive: PathBuf,
        dest: PathBuf,
        #[arg(long = "key")] key_hex: Option<String>,
        #[arg(long = "password")] password: Option<String>,
    },

    /// Verify archive integrity via the Tail Summary.
    Verify {
        archive: PathBuf,
        #[arg(long = "key")] key_hex: Option<String>,
        #[arg(long = "password")] password: Option<String>,
    },

    /// Create an empty archive with embedded metadata.
    Issue {
        out: PathBuf,
        #[arg(long)] label: String,
        #[arg(long, default_value = "")] owner: String,
        #[arg(long, default_value = "")] notes: String,
        #[arg(long = "encrypt-raw")] encrypt_raw_hex: Option<String>,
        /// Derive encryption key from a password (Argon2id).
        #[arg(long)] password: Option<String>,
        #[arg(long)] deterministic: bool,
    },

    #[command(subcommand)]
    /// Inspect chunk maps and stream file content.
    Chunk(ChunkCommands),

    #[command(subcommand)]
    /// CRUD overlay commands (sidecars over an immutable base).
    Crud(CrudCommands),
}
