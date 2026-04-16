pub mod handlers;

use crate::presentation::cli::{ChunkCommands, Cli, Commands, CrudCommands};
use arx_core::error::Result;
use clap::Parser;

pub fn run() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Commands::Pack { out, inputs, deterministic, min_gain, encrypt_raw_hex, password } =>
            handlers::handle_pack(out, inputs, deterministic, min_gain, encrypt_raw_hex, password),

        Commands::List { archive, key_hex, password } =>
            handlers::handle_list(archive, key_hex, password),

        Commands::Extract { archive, dest, key_hex, password } =>
            handlers::handle_extract(archive, dest, key_hex, password),

        Commands::Verify { archive, key_hex, password } =>
            handlers::handle_verify(archive, key_hex, password),

        Commands::Issue { out, label, owner, notes, encrypt_raw_hex, password, deterministic } =>
            handlers::handle_issue(out, label, owner, notes, encrypt_raw_hex, password, deterministic),

        Commands::Chunk(cmd) => match cmd {
            ChunkCommands::Chunks { archive, path, key_hex, password } =>
                handlers::handle_chunk_chunks(archive, path, key_hex, password),
            ChunkCommands::Cat { archive, path, start, len, key_hex, password } =>
                handlers::handle_chunk_cat(archive, path, start, len, key_hex, password),
            ChunkCommands::Get { archive, path, out, start, len, key_hex, password } =>
                handlers::handle_chunk_get(archive, path, out, start, len, key_hex, password),
        },

        Commands::Crud(cmd) => match cmd {
            CrudCommands::Add { archive, src, dst, recursive, mode, mtime, key_hex, password } =>
                handlers::handle_crud_add(archive, src, dst, recursive, mode, mtime, key_hex, password),
            CrudCommands::Rm { archive, path, recursive, key_hex, password } =>
                handlers::handle_crud_rm(archive, path, recursive, key_hex, password),
            CrudCommands::Mv { archive, from, to, key_hex, password } =>
                handlers::handle_crud_mv(archive, from, to, key_hex, password),
            CrudCommands::Ls { archive, prefix, long, key_hex, password } =>
                handlers::handle_crud_ls(archive, prefix, long, key_hex, password),
            CrudCommands::Diff { archive, key_hex, password } =>
                handlers::handle_crud_diff(archive, key_hex, password),
            CrudCommands::Sync { archive, out, deterministic, min_gain, key_hex, password, seal_base } =>
                handlers::handle_crud_sync(archive, out, deterministic, min_gain, key_hex, password, seal_base),
            CrudCommands::Cat { archive, path, key_hex, password } =>
                handlers::handle_crud_cat(archive, path, key_hex, password),
            CrudCommands::Get { archive, path, out, key_hex, password } =>
                handlers::handle_crud_get(archive, path, out, key_hex, password),
        },
    }
}
