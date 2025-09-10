use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(author, version, about = "arxdev CLI (alpha)", long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum ChunkCommands {
    /// Print chunk map for one file
    Chunks {
        archive: PathBuf,
        path: String,
        #[arg(long = "key")]
        key_hex: Option<String>,
        #[arg(long = "key-salt")]
        key_salt_hex: Option<String>,
    },
    /// Stream a file (or range) to stdout
    Cat {
        archive: PathBuf,
        path: String,
        #[arg(long, default_value_t = 0)]
        start: u64,
        #[arg(long)]
        len: Option<u64>,
        #[arg(long = "key")]
        key_hex: Option<String>,
        #[arg(long = "key-salt")]
        key_salt_hex: Option<String>,
    },
    /// Download one file (or range) to an output path
    Get {
        archive: PathBuf,
        path: String,
        out: PathBuf,
        #[arg(long, default_value_t = 0)]
        start: u64,
        #[arg(long)]
        len: Option<u64>,
        #[arg(long = "key")]
        key_hex: Option<String>,
        #[arg(long = "key-salt")]
        key_salt_hex: Option<String>,
    },
}

#[derive(Subcommand)]
pub enum CrudCommands {
    /// Overlay add/put a file or directory (with --recursive) into archive sidecars (delta + journal)
    Add {
        archive: PathBuf,
        /// source file/dir path on disk
        src: PathBuf,
        /// destination path prefix inside the archive (e.g. "/")
        dst: String,
        /// recurse into directories
        #[arg(long)]
        recursive: bool,
        /// file mode (octal). If omitted, inferred from src (on Unix) else 0o644
        #[arg(long)]
        mode: Option<u32>,
        /// mtime in seconds since epoch. If omitted, inferred from src metadata or now()
        #[arg(long)]
        mtime: Option<u64>,
        /// AEAD key (32-byte hex) for encrypted journal+delta
        #[arg(long = "key")]
        key_hex: Option<String>,
        /// AEAD salt (32-byte hex) for nonce derivation
        #[arg(long = "key-salt")]
        key_salt_hex: Option<String>,
    },

    /// Overlay delete (tombstone) a path from the archive
    Rm {
        archive: PathBuf,
        path: String,
        #[arg(long)]
        recursive: bool,
        #[arg(long = "key")]
        key_hex: Option<String>,
        #[arg(long = "key-salt")]
        key_salt_hex: Option<String>,
    },

    /// Overlay rename/move a path within the archive
    Mv {
        archive: PathBuf,
        from: String,
        to: String,
        #[arg(long = "key")]
        key_hex: Option<String>,
        #[arg(long = "key-salt")]
        key_salt_hex: Option<String>,
    },

    /// Overlay list (merged base + sidecars)
    Ls {
        archive: PathBuf,
        /// optional prefix (e.g. "/etc")
        #[arg(long)]
        prefix: Option<String>,
        /// show long format with size/mtime
        #[arg(long)]
        long: bool,
        #[arg(long = "key")]
        key_hex: Option<String>,
        #[arg(long = "key-salt")]
        key_salt_hex: Option<String>,
    },

    /// Compact overlay back into base `.arx` (fold journal+delta into a fresh immutable archive)
    Sync {
        /// existing archive (base .arx + sidecars)
        archive: PathBuf,
        /// output path for the compacted archive (usually overwrite original or write new)
        #[arg(long, default_value = "drive.arx")]
        out: PathBuf,
        /// deterministic mode for compaction
        #[arg(long)]
        deterministic: bool,
        /// min compression gain for zstd before falling back to STORE
        #[arg(long, default_value_t = 0.05)]
        min_gain: f32,
        /// AEAD key/salt to read the overlay (and optionally re-seal the new base)
        #[arg(long = "key")]
        key_hex: Option<String>,
        #[arg(long = "key-salt")]
        key_salt_hex: Option<String>,
        /// When set, re-seal the compacted base with the provided key; else write unencrypted base
        #[arg(long)]
        seal_base: bool,
    },

    Cat {
        archive: PathBuf,
        path: String,
        #[arg(long = "key")]
        key_hex: Option<String>,
        #[arg(long = "key-salt")]
        key_salt_hex: Option<String>,
    },

    /// Download a file from the overlay to an output path
    Get {
        archive: PathBuf,
        path: String,
        out: PathBuf,
        #[arg(long = "key")]
        key_hex: Option<String>,
        #[arg(long = "key-salt")]
        key_salt_hex: Option<String>,
    },
}

#[derive(Subcommand)]
pub enum Commands {
    /// Pack inputs into an ARX archive
    Pack {
        out: PathBuf,
        inputs: Vec<PathBuf>,

        #[arg(long)]
        deterministic: bool,

        #[arg(long, default_value_t = 0.05)]
        min_gain: f32,

        /// 32-byte hex key to enable AEAD (XChaCha20-Poly1305)
        #[arg(long = "encrypt-raw")]
        encrypt_raw_hex: Option<String>,

        /// 32-byte hex salt for nonce derivation (defaults to all-zero)
        #[arg(long = "key-salt")]
        key_salt_hex: Option<String>,
    },

    /// List archive contents
    List {
        archive: PathBuf,

        /// 32-byte hex key for encrypted archives
        #[arg(long = "key")]
        key_hex: Option<String>,

        /// 32-byte hex salt used during pack
        #[arg(long = "key-salt")]
        key_salt_hex: Option<String>,
    },

    /// Extract archive to destination
    Extract {
        archive: PathBuf,
        dest: PathBuf,

        /// 32-byte hex key for encrypted archives
        #[arg(long = "key")]
        key_hex: Option<String>,

        /// 32-byte hex salt used during pack
        #[arg(long = "key-salt")]
        key_salt_hex: Option<String>,
    },

    /// Verify archive integrity (Tail Summary), with optional decryption
    Verify {
        archive: PathBuf,

        /// 32-byte hex key for encrypted archives
        #[arg(long = "key")]
        key_hex: Option<String>,

        /// 32-byte hex salt used during pack
        #[arg(long = "key-salt")]
        key_salt_hex: Option<String>,
    },

    /// Create/issue a fresh archive with root metadata (optionally sealed)
    Issue {
        out: PathBuf,
        /// root name/label to embed
        #[arg(long)]
        label: String,
        /// root owner string (freeform)
        #[arg(long, default_value = "")]
        owner: String,
        /// optional notes
        #[arg(long, default_value = "")]
        notes: String,
        /// seal with AEAD (32B hex key)
        #[arg(long = "encrypt-raw")]
        encrypt_raw_hex: Option<String>,
        /// 32-byte hex salt for nonce derivation (defaults to all-zero)
        #[arg(long = "key-salt")]
        key_salt_hex: Option<String>,
        /// deterministic superblock/manifest timestamps
        #[arg(long)]
        deterministic: bool,
    },

    #[command(subcommand)]
    /// Inspect a file’s chunk map and stream content
    Chunk(ChunkCommands),

    #[command(subcommand)]
    /// CRUD overlay commands (sidecars over immutable base)
    Crud(CrudCommands),
}
