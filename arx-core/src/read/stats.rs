use std::path::Path;

use crate::container::superblock::{FLAG_ENCRYPTED, Superblock};
use crate::error::Result;
use crate::read::opened::Opened;
use crate::stats::Stats;

/// Compute stats for an archive and its optional crud delta sidecar by reading
/// only the superblock and manifest (no chunk decompression).
///
/// Returns compression ratio = stored_bytes / logical_bytes (0..1 for compressed,
/// 1.0 for stored when logical == 0).
pub fn compute_stats(archive_path: &Path, aead_key: Option<[u8; 32]>) -> Result<Stats> {
    let opened = Opened::open(archive_path, aead_key, [0u8; 32])?;

    let mut logical: u64 = 0;
    let mut chunks: u64 = 0;
    for f in &opened.manifest.files {
        logical = logical.saturating_add(f.u_size);
        chunks = chunks.saturating_add(f.chunk_refs.len() as u64);
    }

    let base_size = std::fs::metadata(archive_path)
        .map(|m| m.len())
        .unwrap_or(0);
    let delta_path = archive_path.with_extension("arx.delta");
    let delta_size = std::fs::metadata(&delta_path).map(|m| m.len()).unwrap_or(0);
    let stored = base_size.saturating_add(delta_size);

    Ok(Stats {
        files: opened.manifest.files.len() as u64,
        dirs: opened.manifest.dirs.len() as u64,
        chunks,
        logical_bytes: logical,
        physical_bytes_base: base_size,
        physical_bytes_delta: delta_size,
        compression_ratio: if logical > 0 {
            stored as f32 / logical as f32
        } else {
            1.0
        },
        last_commit_ts: opened.manifest.meta.created.max(0) as u64,
    })
}

/// Read only the `encrypted` flag from the archive superblock — cheaper than
/// opening the full manifest.
pub fn read_encrypted_flag(archive_path: &Path) -> Result<bool> {
    let mut f = std::fs::File::open(archive_path)?;
    let sb = Superblock::read_from(&mut f)?;
    Ok((sb.flags & FLAG_ENCRYPTED) != 0)
}

/// Read the archive's manifest label (if any) without constructing a full
/// Opened (no chunk table bounds check). Returns None if manifest has no label.
pub fn read_manifest_label(archive_path: &Path, aead_key: Option<[u8; 32]>) -> Option<String> {
    // We delegate to Opened for simplicity — cost is one additional chunk table
    // load, which is small. Worth revisiting if list_archives becomes hot.
    let opened = Opened::open(archive_path, aead_key, [0u8; 32]).ok()?;
    opened.manifest.meta.label.clone()
}
