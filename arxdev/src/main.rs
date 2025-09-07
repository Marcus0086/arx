use arx_core::crypto::hex::parse_hex_array;
use arx_core::error::Result;
use arx_core::read::extract::verify;
use arx_core::{ExtractOptions, ListOptions, PackOptions, extract, list, pack};
use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(author, version, about = "arxdev CLI (alpha)", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
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
    }

    Ok(())
}
