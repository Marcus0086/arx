use arx_core::crypto::hex::parse_hex_array;
use arx_core::error::Result;
use arx_core::read::extract::verify;
use arx_core::{ExtractOptions, ListOptions, PackOptions, extract, list, pack};

use arx_core::repo::{ArchiveRepo, OpenParams};
use arx_core::repo_factory::{Backend, open_repo};

use clap::{Parser, Subcommand};
use std::io::Read;
use std::io::Write;
use std::path::PathBuf;

#[derive(Parser)]
#[command(author, version, about = "arxdev CLI (alpha)", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum ChunkCommands {
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
enum Commands {
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

    #[command(subcommand)]
    /// Inspect a file’s chunk map
    Chunk(ChunkCommands),
}

fn repo_from_args(
    archive: PathBuf,
    key_hex: Option<String>,
    key_salt_hex: Option<String>,
) -> Result<Box<dyn ArchiveRepo>> {
    let aead_key = key_hex.map(|h| parse_hex_array::<32>(&h)).transpose()?;
    let key_salt = key_salt_hex
        .map(|h| parse_hex_array::<32>(&h))
        .transpose()?
        .unwrap_or([0u8; 32]);

    let params = OpenParams {
        archive_path: archive,
        aead_key,
        key_salt,
    };
    open_repo(Backend::Fs, params)
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Pack {
            out,
            inputs,
            deterministic,
            min_gain,
            encrypt_raw_hex,
            key_salt_hex,
        } => {
            let refs: Vec<_> = inputs.iter().map(|p| p.as_path()).collect();
            let aead_key = match encrypt_raw_hex {
                Some(hex) => Some(parse_hex_array::<32>(&hex)?),
                None => None,
            };
            let key_salt = match key_salt_hex {
                Some(hex) => parse_hex_array::<32>(&hex)?,
                None => [0u8; 32],
            };

            let opts = PackOptions {
                deterministic,
                min_gain,
                aead_key,
                key_salt,
                ..Default::default()
            };
            pack(&refs, &out, Some(&opts))?;
        }

        Commands::List {
            archive,
            key_hex,
            key_salt_hex,
        } => {
            let aead_key = match key_hex {
                Some(hex) => Some(parse_hex_array::<32>(&hex)?),
                None => None,
            };
            let key_salt = match key_salt_hex {
                Some(hex) => parse_hex_array::<32>(&hex)?,
                None => [0u8; 32],
            };
            let opts = if aead_key.is_some() {
                Some(ListOptions { aead_key, key_salt })
            } else {
                None
            };
            list(&archive, opts.as_ref())?;
        }

        Commands::Extract {
            archive,
            dest,
            key_hex,
            key_salt_hex,
        } => {
            let aead_key = match key_hex {
                Some(hex) => Some(parse_hex_array::<32>(&hex)?),
                None => None,
            };
            let key_salt = match key_salt_hex {
                Some(hex) => parse_hex_array::<32>(&hex)?,
                None => [0u8; 32],
            };
            let opts = if aead_key.is_some() {
                Some(ExtractOptions { aead_key, key_salt })
            } else {
                None
            };
            extract(&archive, &dest, opts.as_ref())?;
        }

        Commands::Verify {
            archive,
            key_hex,
            key_salt_hex,
        } => {
            let aead_key = match key_hex {
                Some(hex) => Some(parse_hex_array::<32>(&hex)?),
                None => None,
            };
            let key_salt = match key_salt_hex {
                Some(hex) => parse_hex_array::<32>(&hex)?,
                None => [0u8; 32],
            };
            let opts = if aead_key.is_some() {
                Some(ExtractOptions { aead_key, key_salt })
            } else {
                None
            };
            verify(&archive, opts.as_ref())?;
            eprintln!("verify: OK");
        }

        Commands::Chunk(chunk_cmd) => match chunk_cmd {
            ChunkCommands::Chunks {
                archive,
                path,
                key_hex,
                key_salt_hex,
            } => {
                let repo = repo_from_args(archive, key_hex, key_salt_hex)?;
                let rows = repo.chunk_map(&path)?;
                for r in rows {
                    println!(
                        "#{:<5} id={:<6} codec={} u={} c={} off={}",
                        r.ordinal, r.id, r.codec, r.u_len, r.c_len, r.data_off
                    );
                }
            }
            ChunkCommands::Cat {
                archive,
                path,
                start,
                len,
                key_hex,
                key_salt_hex,
            } => {
                let repo = repo_from_args(archive, key_hex, key_salt_hex)?;
                let mut reader: Box<dyn Read + Send> = if let Some(l) = len {
                    repo.open_range(&path, start, l)?
                } else {
                    // default to a huge len to stream to EOF (keeps interface simple)
                    repo.open_range(&path, start, u64::MAX / 4)?
                };
                let mut out = std::io::stdout().lock();
                let mut buf = [0u8; 64 * 1024];
                loop {
                    let n = reader.read(&mut buf)?;
                    if n == 0 {
                        break;
                    }
                    out.write_all(&buf[..n])?;
                }
            }
            ChunkCommands::Get {
                archive,
                path,
                out,
                start,
                len,
                key_hex,
                key_salt_hex,
            } => {
                let repo = repo_from_args(archive, key_hex, key_salt_hex)?;
                let mut reader: Box<dyn Read + Send> = if let Some(l) = len {
                    repo.open_range(&path, start, l)?
                } else {
                    repo.open_reader(&path)?
                };
                let mut file = std::fs::File::create(&out)?;
                let mut buf = [0u8; 256 * 1024];
                loop {
                    let n = reader.read(&mut buf)?;
                    if n == 0 {
                        break;
                    }
                    file.write_all(&buf[..n])?;
                }
            }
        },
    }

    Ok(())
}
