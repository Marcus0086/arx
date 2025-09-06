use arx_core::error::ArxError;
use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Subcommand)]
enum Commands {
    Pack { out: PathBuf, inputs: Vec<PathBuf> },
    List { archive: PathBuf },
    Extract { archive: PathBuf, dest: PathBuf },
}

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[clap(subcommand)]
    command: Commands,
}

fn main() -> Result<(), ArxError> {
    let cli = Cli::parse();
    match cli.command {
        Commands::Pack { out, inputs } => {
            let refs: Vec<_> = inputs.iter().map(|p| p.as_path()).collect();
            arx_core::pack(&refs, &out, None)?;
        }
        Commands::List { archive } => {
            arx_core::list(&archive)?;
        }
        Commands::Extract { archive, dest } => {
            arx_core::extract(&archive, &dest)?;
        }
    }
    Ok(())
}
