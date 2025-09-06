use crate::container::manifest::{DirEntry, FileEntry, Manifest, Meta};
use crate::container::superblock::Superblock;
use crate::error::ArxError;
use std::fs::{self, File};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use time::OffsetDateTime;
use walkdir::WalkDir;

#[derive(Clone)]
pub enum Encryption {/* reserved for later */}

#[derive(Clone)]
pub struct PackOptions {
    pub deterministic: bool,
}
impl Default for PackOptions {
    fn default() -> Self {
        Self {
            deterministic: false,
        }
    }
}

pub fn pack(inputs: &[&Path], out: &Path, _opts: Option<&PackOptions>) -> Result<(), ArxError> {
    // 1) collect file/dir entries in a stable order
    let mut files: Vec<PathBuf> = Vec::new();
    let mut dirs: Vec<PathBuf> = Vec::new();

    for root in inputs {
        for e in WalkDir::new(root).follow_links(false) {
            let e =
                e.map_err(|e| ArxError::Io(std::io::Error::new(std::io::ErrorKind::Other, e)))?;
            let p = e.path();
            if e.file_type().is_dir() {
                dirs.push(p.to_path_buf());
            } else if e.file_type().is_file() {
                files.push(p.to_path_buf());
            }
            // symlinks: skip in alpha
        }
    }
    dirs.sort();
    files.sort();

    // 2) create manifest skeleton
    let created = OffsetDateTime::now_utc().unix_timestamp();
    let mut manifest = Manifest {
        files: Vec::with_capacity(files.len()),
        dirs: Vec::with_capacity(dirs.len()),
        meta: Meta {
            created,
            tool: "arx-core/alpha".to_string(),
        },
    };

    for d in &dirs {
        let md = fs::metadata(d)?;
        let m = mode_from(&md);
        let t = mtime_from(&md);
        manifest.dirs.push(DirEntry {
            path: rel_display(d, inputs)?,
            mode: m,
            mtime: t,
        });
    }

    // We'll fill data_off later after we know manifest length.
    let mut tmp_files: Vec<(PathBuf, FileEntry)> = Vec::new();
    for fpath in &files {
        let md = fs::metadata(fpath)?;
        let m = mode_from(&md);
        let t = mtime_from(&md);
        tmp_files.push((
            fpath.clone(),
            FileEntry {
                path: rel_display(fpath, inputs)?,
                mode: m,
                mtime: t,
                size: md.len(),
                data_off: 0, // to be patched
            },
        ));
    }

    // 3) open output and reserve superblock (fixed size)
    let mut out_f = File::create(out)?;

    // placeholder superblock; we'll rewrite once we know manifest_len & data_off
    let placeholder = Superblock {
        version: crate::container::superblock::VERSION,
        manifest_len: 0,
        data_off: 0,
    };
    placeholder.write_to(&mut out_f)?; // write 6 + 2 + 8 + 8 bytes = 24 bytes

    // 4) write a provisional manifest (empty files list) to compute offsets
    //    Instead, simpler: compute data_off = current_pos + encoded_manifest_len
    //    So first, fill manifest.files with entries having data_off=0 and CBOR-encode to get len.
    manifest.files = tmp_files.iter().map(|(_, fe)| fe).cloned().collect();
    let mut manifest_buf = Vec::new();
    ciborium::ser::into_writer(&manifest, &mut manifest_buf)
        .map_err(|e| ArxError::Format(format!("manifest encode: {e}")))?;
    let mut manifest_len = manifest_buf.len() as u64;
    let mut data_off = 24;
    // data section starts immediately after superblock + manifest
    loop {
        // data section starts right after superblock + manifest
        data_off = 24 + manifest_len;

        // patch per-file offsets based on current data_off
        let mut cursor = data_off;
        for fe in manifest.files.iter_mut() {
            fe.data_off = cursor;
            cursor += fe.size;
        }

        // re-encode manifest and check if its length changed
        manifest_buf.clear();
        ciborium::ser::into_writer(&manifest, &mut manifest_buf)
            .map_err(|e| ArxError::Format(format!("manifest encode: {e}")))?;

        let new_len = manifest_buf.len() as u64;
        if new_len == manifest_len {
            break; // stabilized
        }
        manifest_len = new_len;
    }
    // 6) write final manifest
    out_f.seek(SeekFrom::Start(24))?;
    out_f.write_all(&manifest_buf)?;
    let pos = out_f.stream_position()?;
    assert_eq!(
        pos,
        data_off,
        "manifest size mismatch: wrote {}, expected {}",
        pos - 24,
        manifest_len
    );

    // 7) stream file bytes in the same order
    let mut buf = vec![0u8; 1 << 16];
    for (idx, fe) in manifest.files.iter().enumerate() {
        let (src_path, _fe_template) = &tmp_files[idx]; // src_path = original filesystem path
        let mut src = File::open(src_path)?;
        out_f.seek(SeekFrom::Start(fe.data_off))?;
        loop {
            let n = src.read(&mut buf)?;
            if n == 0 {
                break;
            }
            out_f.write_all(&buf[..n])?;
        }
    }

    // 8) rewrite superblock with correct lengths/offsets
    out_f.seek(SeekFrom::Start(0))?;
    let sb = Superblock {
        version: crate::container::superblock::VERSION,
        manifest_len,
        data_off,
    };
    sb.write_to(&mut out_f)?;

    Ok(())
}

// helpers
fn mode_from(_md: &std::fs::Metadata) -> u32 {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        _md.permissions().mode()
    }
    #[cfg(not(unix))]
    {
        0o100644
    }
}
fn mtime_from(md: &std::fs::Metadata) -> i64 {
    md.modified()
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}
fn rel_display(path: &Path, roots: &[&Path]) -> Result<String, ArxError> {
    // find first root that is a prefix; else emit as relative string
    for r in roots {
        if let Ok(p) = path.strip_prefix(r) {
            return Ok(p.to_string_lossy().to_string());
        }
    }
    Ok(path.to_string_lossy().to_string())
}
