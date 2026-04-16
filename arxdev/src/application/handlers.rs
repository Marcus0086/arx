use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use arx_core::container::superblock::Superblock;
use arx_core::crud::CrudArchive;
use arx_core::crypto::hex::parse_hex_array;
use arx_core::crypto::kdf::derive_key;
use arx_core::error::Result;
use arx_core::read::extract::verify;
use arx_core::repo::{ArchiveRepo, OpenParams};
use arx_core::repo_factory::{Backend, open_repo};
use arx_core::{ExtractOptions, ListOptions, PackOptions, extract, list, pack};

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

// ── Key resolution ────────────────────────────────────────────────────────────

/// Resolve a raw 32-byte key from either a hex string or a password.
/// When using a password, the archive's superblock is read to obtain the
/// stored kdf_salt used during nonce derivation.
fn resolve_key(
    archive: &Path,
    key_hex: Option<&str>,
    password: Option<&str>,
) -> Result<Option<[u8; 32]>> {
    if let Some(hex) = key_hex {
        return Ok(Some(parse_hex_array::<32>(hex)?));
    }
    if let Some(pw) = password {
        let mut f = std::fs::File::open(archive)?;
        let sb = Superblock::read_from(&mut f)?;
        return Ok(Some(derive_key(pw, &sb.kdf_salt)));
    }
    Ok(None)
}

fn repo_from_args(
    archive: PathBuf,
    key_hex: Option<String>,
    password: Option<String>,
) -> Result<Box<dyn ArchiveRepo>> {
    let aead_key = resolve_key(
        &archive,
        key_hex.as_deref(),
        password.as_deref(),
    )?;
    let params = OpenParams {
        archive_path: archive,
        aead_key,
        key_salt: [0u8; 32], // salt is read from the superblock inside Opened::open
    };
    open_repo(Backend::Fs, params)
}

fn crud_key(
    archive: &Path,
    key_hex: Option<String>,
    password: Option<String>,
) -> Result<Option<[u8; 32]>> {
    resolve_key(archive, key_hex.as_deref(), password.as_deref())
}

fn infer_mode(src: &Path, override_mode: Option<u32>) -> u32 {
    if let Some(m) = override_mode {
        return m;
    }
    #[cfg(unix)]
    {
        if let Ok(md) = std::fs::metadata(src) {
            return md.permissions().mode();
        }
    }
    0o644
}

fn infer_mtime(src: &Path, override_mtime: Option<u64>) -> u64 {
    if let Some(t) = override_mtime {
        return t;
    }
    if let Ok(md) = std::fs::metadata(src) {
        if let Ok(st) = md.modified() {
            if let Ok(d) = st.duration_since(UNIX_EPOCH) {
                return d.as_secs();
            }
        }
    }
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

// ── Command handlers ──────────────────────────────────────────────────────────

pub fn handle_pack(
    out: PathBuf,
    inputs: Vec<PathBuf>,
    deterministic: bool,
    min_gain: f32,
    encrypt_raw_hex: Option<String>,
    password: Option<String>,
) -> Result<()> {
    let refs: Vec<_> = inputs.iter().map(|p| p.as_path()).collect();
    let aead_key = encrypt_raw_hex
        .map(|hex| parse_hex_array::<32>(&hex))
        .transpose()?;
    eprintln!("packing {} input(s) → {}", inputs.len(), out.display());
    let opts = PackOptions {
        deterministic,
        min_gain,
        aead_key,
        password,
        ..Default::default()
    };
    pack(&refs, &out, Some(&opts))?;
    eprintln!("pack: done → {}", out.display());
    Ok(())
}

pub fn handle_list(
    archive: PathBuf,
    key_hex: Option<String>,
    password: Option<String>,
) -> Result<()> {
    let aead_key = resolve_key(&archive, key_hex.as_deref(), password.as_deref())?;
    let opts = aead_key.map(|k| ListOptions { aead_key: Some(k), key_salt: [0u8; 32] });
    list(&archive, opts.as_ref())
}

pub fn handle_extract(
    archive: PathBuf,
    dest: PathBuf,
    key_hex: Option<String>,
    password: Option<String>,
) -> Result<()> {
    let aead_key = resolve_key(&archive, key_hex.as_deref(), password.as_deref())?;
    let opts = aead_key.map(|k| ExtractOptions { aead_key: Some(k), key_salt: [0u8; 32], password: None });
    extract(&archive, &dest, opts.as_ref())
}

pub fn handle_verify(
    archive: PathBuf,
    key_hex: Option<String>,
    password: Option<String>,
) -> Result<()> {
    let aead_key = resolve_key(&archive, key_hex.as_deref(), password.as_deref())?;
    let opts = aead_key.map(|k| ExtractOptions { aead_key: Some(k), key_salt: [0u8; 32], password: None });
    verify(&archive, opts.as_ref())?;
    eprintln!("verify: OK");
    Ok(())
}

pub fn handle_issue(
    out: PathBuf,
    label: String,
    owner: String,
    notes: String,
    encrypt_raw_hex: Option<String>,
    password: Option<String>,
    deterministic: bool,
) -> Result<()> {
    let aead_key = encrypt_raw_hex
        .map(|hex| parse_hex_array::<32>(&hex))
        .transpose()?;
    CrudArchive::issue_archive(
        &out, &label, &owner, &notes, aead_key, [0u8; 32], deterministic,
    )?;
    // If password given, re-derive key won't work here since archive just created;
    // password is handled via PackOptions.password in issue_archive already
    let _ = password; // future: pass through to PackOptions
    eprintln!("issue: created {} (label=\"{}\")", out.display(), label);
    Ok(())
}

pub fn handle_chunk_chunks(
    archive: PathBuf,
    path: String,
    key_hex: Option<String>,
    password: Option<String>,
) -> Result<()> {
    let repo = repo_from_args(archive, key_hex, password)?;
    let rows = repo.chunk_map(&path)?;
    for r in rows {
        println!(
            "#{:<5} id={:<6} codec={} u={} c={} off={}",
            r.ordinal, r.id, r.codec, r.u_len, r.c_len, r.data_off
        );
    }
    Ok(())
}

pub fn handle_chunk_cat(
    archive: PathBuf,
    path: String,
    start: u64,
    len: Option<u64>,
    key_hex: Option<String>,
    password: Option<String>,
) -> Result<()> {
    let repo = repo_from_args(archive, key_hex, password)?;
    let mut reader: Box<dyn Read + Send> = if let Some(l) = len {
        repo.open_range(&path, start, l)?
    } else {
        repo.open_range(&path, start, u64::MAX / 4)?
    };
    let mut out = std::io::stdout().lock();
    let mut buf = [0u8; 64 * 1024];
    loop {
        let n = reader.read(&mut buf)?;
        if n == 0 { break; }
        out.write_all(&buf[..n])?;
    }
    Ok(())
}

pub fn handle_chunk_get(
    archive: PathBuf,
    path: String,
    out: PathBuf,
    start: u64,
    len: Option<u64>,
    key_hex: Option<String>,
    password: Option<String>,
) -> Result<()> {
    let repo = repo_from_args(archive, key_hex, password)?;
    let mut reader: Box<dyn Read + Send> = if let Some(l) = len {
        repo.open_range(&path, start, l)?
    } else {
        repo.open_reader(&path)?
    };
    let mut file = std::fs::File::create(&out)?;
    let mut buf = [0u8; 256 * 1024];
    loop {
        let n = reader.read(&mut buf)?;
        if n == 0 { break; }
        file.write_all(&buf[..n])?;
    }
    Ok(())
}

pub fn handle_crud_add(
    archive: PathBuf,
    src: PathBuf,
    dst: String,
    recursive: bool,
    mode: Option<u32>,
    mtime: Option<u64>,
    key_hex: Option<String>,
    password: Option<String>,
) -> Result<()> {
    let aead_key = crud_key(&archive, key_hex, password)?;
    let mut arc = CrudArchive::open_with_crypto(&archive, aead_key, [0u8; 32])?;
    if recursive && src.is_dir() {
        let base = src.clone();
        let dst_root = Path::new(&dst);
        for entry in walkdir::WalkDir::new(&src).into_iter().filter_map(|e| e.ok()) {
            if entry.file_type().is_file() {
                let p = entry.path().to_path_buf();
                let rel = p.strip_prefix(&base).unwrap();
                let inside = dst_root.join(rel).to_string_lossy().to_string();
                let m = infer_mode(&p, mode);
                let t = infer_mtime(&p, mtime);
                arc.put_file(&p, &inside, m, t)?;
                eprintln!("add: {} -> {inside}", p.display());
            }
        }
    } else {
        let m = infer_mode(&src, mode);
        let t = infer_mtime(&src, mtime);
        arc.put_file(&src, &dst, m, t)?;
        eprintln!("add: {} -> {dst}", src.display());
    }
    Ok(())
}

pub fn handle_crud_rm(
    archive: PathBuf,
    path: String,
    recursive: bool,
    key_hex: Option<String>,
    password: Option<String>,
) -> Result<()> {
    let aead_key = crud_key(&archive, key_hex, password)?;
    let mut arc = CrudArchive::open_with_crypto(&archive, aead_key, [0u8; 32])?;
    if recursive {
        arc.delete_path_recursive(&path)?;
    } else {
        arc.delete_path(&path)?;
    }
    eprintln!("rm: {path}");
    Ok(())
}

pub fn handle_crud_mv(
    archive: PathBuf,
    from: String,
    to: String,
    key_hex: Option<String>,
    password: Option<String>,
) -> Result<()> {
    let aead_key = crud_key(&archive, key_hex, password)?;
    let mut arc = CrudArchive::open_with_crypto(&archive, aead_key, [0u8; 32])?;
    arc.rename(&from, &to)?;
    eprintln!("mv: {from} -> {to}");
    Ok(())
}

pub fn handle_crud_ls(
    archive: PathBuf,
    prefix: Option<String>,
    long: bool,
    key_hex: Option<String>,
    password: Option<String>,
) -> Result<()> {
    let aead_key = crud_key(&archive, key_hex, password)?;
    let arc = CrudArchive::open_with_crypto(&archive, aead_key, [0u8; 32])?;
    let iter = arc.index.by_path.iter().filter(|(p, _)| {
        prefix.as_ref().map(|pref| p.starts_with(pref)).unwrap_or(true)
    });
    if long {
        for (p, e) in iter {
            println!("{:>12}  {:>10}  {p}", e.size, e.mtime);
        }
    } else {
        for (p, _) in iter {
            println!("{p}");
        }
    }
    Ok(())
}

pub fn handle_crud_diff(
    archive: PathBuf,
    key_hex: Option<String>,
    password: Option<String>,
) -> Result<()> {
    let aead_key = crud_key(&archive, key_hex, password)?;
    let arc = CrudArchive::open_with_crypto(&archive, aead_key, [0u8; 32])?;
    let entries = arc.diff();
    if entries.is_empty() {
        eprintln!("diff: no changes");
    } else {
        for e in entries {
            println!("{} {}", e.kind, e.path);
        }
    }
    Ok(())
}

pub fn handle_crud_sync(
    archive: PathBuf,
    out: Option<PathBuf>,
    deterministic: bool,
    min_gain: f32,
    key_hex: Option<String>,
    password: Option<String>,
    seal_base: bool,
) -> Result<()> {
    let aead_key = crud_key(&archive, key_hex, password)?;
    let out_path = out.as_deref();
    let display_out = out.as_ref()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|| format!("{} (in-place)", archive.display()));
    CrudArchive::sync_to_base(
        &archive,
        out_path,
        deterministic,
        min_gain,
        aead_key,
        [0u8; 32],
        seal_base,
    )?;
    eprintln!("sync: {} -> {display_out}", archive.display());
    Ok(())
}

pub fn handle_crud_cat(
    archive: PathBuf,
    path: String,
    key_hex: Option<String>,
    password: Option<String>,
) -> Result<()> {
    let aead_key = crud_key(&archive, key_hex, password)?;
    let arc = CrudArchive::open_with_crypto(&archive, aead_key, [0u8; 32])?;
    let mut r = arc.open_reader(&path)?;
    let mut out = std::io::stdout().lock();
    let mut buf = [0u8; 64 * 1024];
    loop {
        let n = r.read(&mut buf)?;
        if n == 0 { break; }
        out.write_all(&buf[..n])?;
    }
    Ok(())
}

pub fn handle_crud_get(
    archive: PathBuf,
    path: String,
    out: PathBuf,
    key_hex: Option<String>,
    password: Option<String>,
) -> Result<()> {
    let aead_key = crud_key(&archive, key_hex, password)?;
    let arc = CrudArchive::open_with_crypto(&archive, aead_key, [0u8; 32])?;
    let mut r = arc.open_reader(&path)?;
    let mut file = std::fs::File::create(&out)?;
    let mut buf = [0u8; 256 * 1024];
    loop {
        let n = r.read(&mut buf)?;
        if n == 0 { break; }
        file.write_all(&buf[..n])?;
    }
    Ok(())
}
