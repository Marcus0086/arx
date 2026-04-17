use std::fs::File;
use std::io::{Cursor, Read};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::chunking::fastcdc::{ChunkParams, StreamingChunker};
use crate::codec::zstdc::ZstdCompressor;
use crate::codec::{CodecId, Compressor, get_decoder_u8};
use crate::container::delta::DeltaStore;
use crate::container::journal::{ChunkRef, EncMode, Journal, Loc, LogRecord};
use crate::error::{ArxError, Result};
use crate::index::inmem::InMemIndex;
use crate::read::opened::Opened;
use crate::{PackOptions, pack};

pub struct DiffEntry {
    /// "A" = added, "D" = deleted, "M" = modified, "R" = renamed
    pub kind: &'static str,
    pub path: String,
    /// For renames: the original path.
    pub from: Option<String>,
}

pub struct CrudArchive {
    pub base_path: PathBuf,
    pub log_path: PathBuf,
    pub delta_path: PathBuf,
    pub index: InMemIndex,
    pub journal: Journal,
    pub delta: DeltaStore,
    /// Opened view of the base archive (for reading Base-located chunks).
    base_opened: Arc<Opened>,
    min_gain: f32,
}

impl CrudArchive {
    /// Open overlay with optional AEAD encryption.
    pub fn open_with_crypto(
        base: &Path,
        aead_key: Option<[u8; 32]>,
        key_salt: [u8; 32],
    ) -> Result<Self> {
        let base_path = base.to_path_buf();
        let log_path = with_ext(base, "arx.log");
        let delta_path = with_ext(base, "arx.delta");

        let enc = if let Some(key) = aead_key {
            EncMode::Aead {
                key,
                salt: key_salt,
            }
        } else {
            EncMode::Plain
        };

        // Open base archive and seed the index from it
        let base_opened = Opened::open(base, aead_key, key_salt)?;
        let mut index = InMemIndex::from_base(&base_opened)?;

        // Replay journal on top
        let mut journal = Journal::open(&log_path, enc)?;
        {
            let mut it = journal.iter()?;
            for rec in &mut it {
                index.apply(&rec?);
            }
        }

        let delta = DeltaStore::open(&delta_path, enc)?;

        Ok(Self {
            base_path,
            log_path,
            delta_path,
            index,
            journal,
            delta,
            base_opened: Arc::new(base_opened),
            min_gain: 0.05,
        })
    }

    pub fn open(base: &Path) -> Result<Self> {
        Self::open_with_crypto(base, None, [0u8; 32])
    }

    /// Add a file to the overlay using FastCDC chunking + Zstd compression.
    pub fn put_file<P: AsRef<Path>>(
        &mut self,
        src: P,
        dst_path: &str,
        mode: u32,
        mtime: u64,
    ) -> Result<()> {
        let src = src.as_ref();
        let mut f = File::open(src)?;
        let mut chunker = StreamingChunker::new(ChunkParams::default());
        let zstd = ZstdCompressor;
        let min_gain = self.min_gain;

        let mut chunk_refs: Vec<ChunkRef> = Vec::new();
        let mut total = 0u64;

        loop {
            let mut buf = Vec::new();
            let n = chunker.next_chunk(&mut f, &mut buf)?;
            if n == 0 {
                break;
            }
            total += n as u64;

            // Hash uncompressed chunk
            let hash = *blake3::hash(&buf).as_bytes();

            // Trial compress
            let mut compressed = Vec::with_capacity(n);
            zstd.compress(&mut buf.as_slice(), &mut compressed, 3)?;

            let (payload, codec) =
                if (n as f64 - compressed.len() as f64) >= n as f64 * min_gain as f64 {
                    (compressed, CodecId::Zstd)
                } else {
                    (buf, CodecId::Store)
                };

            let (off, len) = self.delta.append_frame(&payload)?;
            chunk_refs.push(ChunkRef {
                loc: Loc::Delta,
                off,
                len,
                codec,
                blake3: hash,
            });
        }

        let rec = LogRecord::Put {
            path: dst_path.to_string(),
            mode,
            mtime,
            size: total,
            chunks: chunk_refs.clone(),
        };
        self.journal.append(&rec)?;
        self.index.apply(&rec);
        Ok(())
    }

    pub fn delete_path(&mut self, path: &str) -> Result<()> {
        let rec = LogRecord::Delete {
            path: path.to_string(),
        };
        self.journal.append(&rec)?;
        self.index.apply(&rec);
        Ok(())
    }

    pub fn delete_path_recursive(&mut self, path: &str) -> Result<()> {
        // Collect all overlay paths that are under `path`
        let prefix = if path.ends_with('/') {
            path.to_string()
        } else {
            format!("{path}/")
        };
        let to_delete: Vec<String> = self
            .index
            .by_path
            .keys()
            .filter(|p| *p == path || p.starts_with(&prefix))
            .cloned()
            .collect();

        for p in to_delete {
            let rec = LogRecord::Delete { path: p };
            self.journal.append(&rec)?;
            self.index.apply(&rec);
        }
        Ok(())
    }

    pub fn rename(&mut self, from: &str, to: &str) -> Result<()> {
        let rec = LogRecord::Rename {
            from: from.to_string(),
            to: to.to_string(),
        };
        self.journal.append(&rec)?;
        self.index.apply(&rec);
        Ok(())
    }

    /// Open a reader for a path in the merged overlay (base + journal).
    ///
    /// - Files that exist only in the base: delegates to `Opened::open_reader` (streaming, no buffer).
    /// - Files that exist only in delta (added via `put_file`): decompresses and buffers.
    /// - Mixed base+delta files: not yet supported; returns an error suggesting `sync` first.
    pub fn open_reader(&self, path: &str) -> Result<Box<dyn Read + Send + '_>> {
        let entry = self.index.by_path.get(path).ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("path not found: {path}"),
            )
        })?;

        let all_base = entry.chunks.iter().all(|c| c.loc == Loc::Base);
        let all_delta = entry.chunks.iter().all(|c| c.loc == Loc::Delta);

        if all_base {
            // Delegate to the base archive reader — handles decrypt+decompress efficiently
            let r = self.base_opened.open_reader(path)?;
            return Ok(Box::new(r));
        }

        if all_delta {
            let mut out = Vec::with_capacity(entry.size as usize);
            for c in &entry.chunks {
                let mut r = self.delta.read_frame(c.off, c.len)?;
                let mut compressed = Vec::new();
                std::io::copy(&mut r, &mut compressed)?;
                // Decompress based on stored codec
                let plain = decompress_bytes(&compressed, c.codec)?;
                out.extend_from_slice(&plain);
            }
            return Ok(Box::new(Cursor::new(out)));
        }

        Err(ArxError::Format(
            "mixed Base/Delta files are not yet supported in open_reader — run `crud sync` first to compact the overlay".into()
        ))
    }

    /// Return a diff between the current overlay state and the original base.
    pub fn diff(&self) -> Vec<DiffEntry> {
        use std::collections::HashSet;

        let base_paths: HashSet<&str> = self
            .base_opened
            .manifest
            .files
            .iter()
            .map(|fe| fe.path.as_str())
            .collect();

        let overlay_paths: HashSet<&str> = self.index.by_path.keys().map(|s| s.as_str()).collect();

        let mut entries = Vec::new();

        // Added (in overlay, not in base)
        for path in overlay_paths.difference(&base_paths) {
            entries.push(DiffEntry {
                kind: "A",
                path: path.to_string(),
                from: None,
            });
        }

        // Deleted (in base, not in overlay)
        for path in base_paths.difference(&overlay_paths) {
            entries.push(DiffEntry {
                kind: "D",
                path: path.to_string(),
                from: None,
            });
        }

        // Modified (in both, but overlay has delta chunks)
        for path in overlay_paths.intersection(&base_paths) {
            if let Some(entry) = self.index.by_path.get(*path) {
                let has_delta = entry.chunks.iter().any(|c| c.loc == Loc::Delta);
                if has_delta {
                    entries.push(DiffEntry {
                        kind: "M",
                        path: path.to_string(),
                        from: None,
                    });
                }
            }
        }

        entries.sort_by(|a, b| a.path.cmp(&b.path));
        entries
    }

    /// Compact the overlay into a fresh immutable base archive at `out`.
    /// If `out` is None, overwrites the original archive in-place.
    pub fn sync_to_base(
        archive: &Path,
        out: Option<&Path>,
        deterministic: bool,
        min_gain: f32,
        aead_key: Option<[u8; 32]>,
        key_salt: [u8; 32],
        seal_base: bool,
    ) -> Result<()> {
        let arc = CrudArchive::open_with_crypto(archive, aead_key, key_salt)?;

        let tmp = tempfile::tempdir()?;
        for (path, _) in arc.index.by_path.iter() {
            let abs = tmp.path().join(path.trim_start_matches('/'));
            if let Some(parent) = abs.parent() {
                std::fs::create_dir_all(parent)?;
            }
            let mut w = std::fs::File::create(&abs)?;
            let mut r = arc.open_reader(path)?;
            std::io::copy(&mut r, &mut w)?;
        }

        let inputs = vec![tmp.path().to_path_buf()];
        let refs: Vec<&Path> = inputs.iter().map(|p| p.as_path()).collect();
        let opts = PackOptions {
            deterministic,
            min_gain,
            aead_key: if seal_base { aead_key } else { None },
            key_salt,
            ..Default::default()
        };

        // Determine output path
        let final_out = out
            .map(|p| p.to_path_buf())
            .unwrap_or_else(|| archive.to_path_buf());
        let tmp_out = final_out.with_extension("arx.tmp");
        pack(&refs, &tmp_out, Some(&opts))?;

        // Atomic rename if writing in-place
        std::fs::rename(&tmp_out, &final_out)?;
        Ok(())
    }

    /// Create an empty archive with first-class metadata (no fake files).
    pub fn issue_archive(
        out: &Path,
        label: &str,
        owner: &str,
        notes: &str,
        aead_key: Option<[u8; 32]>,
        key_salt: [u8; 32],
        deterministic: bool,
    ) -> Result<()> {
        let tmp = tempfile::tempdir()?;
        // No files — just create empty archive with metadata in the manifest
        let empty: Vec<&Path> = vec![tmp.path()];
        let opts = PackOptions {
            deterministic,
            min_gain: 0.05,
            aead_key,
            key_salt,
            meta_label: if label.is_empty() {
                None
            } else {
                Some(label.to_string())
            },
            meta_owner: if owner.is_empty() {
                None
            } else {
                Some(owner.to_string())
            },
            meta_notes: if notes.is_empty() {
                None
            } else {
                Some(notes.to_string())
            },
            ..Default::default()
        };
        pack(&empty, out, Some(&opts))?;
        Ok(())
    }
}

// ── Private helpers ───────────────────────────────────────────────────────────

fn decompress_bytes(compressed: &[u8], codec: CodecId) -> Result<Vec<u8>> {
    let mut out = Vec::new();
    get_decoder_u8(codec as u8)?.decompress(&mut compressed.as_ref(), &mut out)?;
    Ok(out)
}

fn with_ext(base: &Path, ext: &str) -> PathBuf {
    let mut p = PathBuf::from(base);
    if let Some(os) = p.file_name() {
        if let Some(name) = os.to_str() {
            if name.ends_with(".arx") {
                let stem = &name[..name.len() - 4];
                p.set_file_name(format!("{stem}.{ext}"));
                return p;
            }
        }
    }
    p.set_extension(ext);
    p
}
