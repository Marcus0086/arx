use std::fs::File;
use std::io::Cursor;
use std::io::Read;
use std::path::{Path, PathBuf};

use crate::codec::CodecId;
use crate::container::delta::DeltaStore;
use crate::container::journal::{ChunkRef, EncMode, Journal, Loc, LogRecord};
use crate::error::Result;
use crate::index::inmem::InMemIndex;
use crate::{PackOptions, pack};

pub struct CrudArchive {
    pub base_path: PathBuf,
    pub log_path: PathBuf,
    pub delta_path: PathBuf,
    pub index: InMemIndex,
    pub journal: Journal,
    pub delta: DeltaStore,
}

impl CrudArchive {
    /// Open overlay; when `aead_key` is Some, both sidecars are AEAD-sealed.
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

        let mut journal = Journal::open(&log_path, enc)?;
        let mut index = InMemIndex::from_base()?; // TODO: merge base once wired
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
        })
    }

    pub fn open(base: &Path) -> Result<Self> {
        Self::open_with_crypto(base, None, [0u8; 32])
    }

    /// Minimal PUT: single-frame STORE; FastCDC+Zstd can replace later.
    pub fn put_file<P: AsRef<Path>>(
        &mut self,
        src: P,
        dst_path: &str,
        mode: u32,
        mtime: u64,
    ) -> Result<()> {
        let mut f = File::open(src.as_ref())?;
        let mut hasher = blake3::Hasher::new();
        let mut frame = Vec::with_capacity(64 * 1024);
        let mut buf = [0u8; 64 * 1024];
        let mut total = 0u64;
        loop {
            let n = f.read(&mut buf)?;
            if n == 0 {
                break;
            }
            hasher.update(&buf[..n]);
            frame.extend_from_slice(&buf[..n]);
            total += n as u64;
        }
        let hash = *hasher.finalize().as_bytes();

        let (off, len) = self.delta.append_frame(&frame)?;
        let chunks = vec![ChunkRef {
            loc: Loc::Delta,
            off,
            len,
            codec: CodecId::Store,
            blake3: hash,
        }];
        let rec = LogRecord::Put {
            path: dst_path.to_string(),
            mode,
            mtime,
            size: total,
            chunks: chunks.clone(),
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
        let rec = LogRecord::Delete {
            path: path.to_string(),
        };
        for entry in walkdir::WalkDir::new(path)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let p = entry.path().to_path_buf();
            let rel = p.strip_prefix(&path).unwrap();
            let inside = rel.to_string_lossy().to_string();
            let rec = LogRecord::Delete {
                path: inside.to_string(),
            };
            self.journal.append(&rec)?;
            self.index.apply(&rec);
        }
        self.journal.append(&rec)?;
        self.index.apply(&rec);
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

    /// Open a reader over the *overlay* content for `path` (Delta chunks supported).
    /// Returns Err if any chunk points to Base (until base reader is wired).
    pub fn open_reader(&self, path: &str) -> Result<Box<dyn Read + Send>> {
        let entry =
            self.index.by_path.get(path).ok_or_else(|| {
                std::io::Error::new(std::io::ErrorKind::NotFound, "path not found")
            })?;

        // For now, require all chunks to be Delta.
        for c in &entry.chunks {
            if matches!(c.loc, Loc::Base) {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::Unsupported,
                    "overlay reader for Base chunks not wired yet",
                )
                .into());
            }
        }

        // Chain delta frames
        let mut out = Vec::with_capacity(entry.size as usize);
        for c in &entry.chunks {
            let mut r = self.delta.read_frame(c.off, c.len)?;
            std::io::copy(&mut r, &mut out)?;
        }
        Ok(Box::new(Cursor::new(out)))
    }

    /// Compact overlay into a fresh base archive at `out`.
    pub fn sync_to_base(
        archive: &Path,
        out: &Path,
        deterministic: bool,
        min_gain: f32,
        aead_key: Option<[u8; 32]>,
        key_salt: [u8; 32],
        seal_base: bool,
    ) -> Result<()> {
        let arc = CrudArchive::open_with_crypto(archive, aead_key, key_salt)?;

        let tmp = tempfile::tempdir()?;
        for (path, entry) in arc.index.by_path.iter() {
            let abs = tmp.path().join(&path.trim_start_matches('/'));
            if let Some(parent) = abs.parent() {
                std::fs::create_dir_all(parent)?;
            }
            let mut w = std::fs::File::create(&abs)?;
            for c in &entry.chunks {
                match c.loc {
                    Loc::Delta => {
                        let mut r = arc.delta.read_frame(c.off, c.len)?;
                        std::io::copy(&mut r, &mut w)?;
                    }
                    Loc::Base => {
                        return Err(crate::error::ArxError::Format(
                            "sync_to_base: Base chunks require base reader".into(),
                        ));
                    }
                }
            }
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
        pack(&refs, out, Some(&opts))?;
        Ok(())
    }

    /// Issue an empty archive embedding root metadata as a small marker file.
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
        let meta_path = tmp.path().join("__arx_root_meta.txt");
        std::fs::write(
            &meta_path,
            format!("label={}\nowner={}\nnotes={}\n", label, owner, notes),
        )?;
        let inputs = vec![tmp.path().to_path_buf()];
        let refs: Vec<&Path> = inputs.iter().map(|p| p.as_path()).collect();
        let opts = PackOptions {
            deterministic,
            min_gain: 0.05,
            aead_key,
            key_salt,
            ..Default::default()
        };
        pack(&refs, out, Some(&opts))?;
        Ok(())
    }
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
