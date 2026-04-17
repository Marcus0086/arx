use std::path::{Component, Path, PathBuf};
use std::sync::Arc;

use axum::{
    Json,
    extract::{DefaultBodyLimit, Multipart, Path as AxumPath, State},
    http::{HeaderMap, StatusCode},
};
use dashmap::DashMap;
use serde_json::{Value, json};
use tokio::sync::Mutex;

use crate::auth::extract_identity;
use crate::db::AuthDb;
use crate::store::ArchiveStore;
use arx_core::crud::CrudArchive;

/// Max upload size per request (2 GiB). Browsers typically hit memory limits before this.
pub const MAX_UPLOAD_BYTES: usize = 2 * 1024 * 1024 * 1024;

#[derive(Clone)]
pub struct UploadState {
    pub store: Arc<ArchiveStore>,
    pub db: Arc<AuthDb>,
    pub archive_locks: Arc<DashMap<String, Arc<Mutex<()>>>>,
}

pub fn body_limit() -> DefaultBodyLimit {
    DefaultBodyLimit::max(MAX_UPLOAD_BYTES)
}

/// Reject any path component that isn't a plain filename segment.
/// Returns None if the path contains `..`, absolute roots, or is empty.
fn sanitize_path(raw: &str) -> Option<PathBuf> {
    let mut out = PathBuf::new();
    for comp in Path::new(raw).components() {
        match comp {
            Component::Normal(c) => out.push(c),
            _ => return None,
        }
    }
    if out.as_os_str().is_empty() {
        None
    } else {
        Some(out)
    }
}

/// POST /api/upload/{archive_id}
///
/// Accepts multipart/form-data. Each field is one file.
/// Field name is used as the archive path; `filename` is the fallback.
pub async fn handle_upload(
    State(state): State<UploadState>,
    AxumPath(archive_id): AxumPath<String>,
    headers: HeaderMap,
    mut multipart: Multipart,
) -> Result<Json<Value>, (StatusCode, String)> {
    let token = headers
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.strip_prefix("Bearer "))
        .ok_or((StatusCode::UNAUTHORIZED, "missing bearer token".into()))?;

    let (tenant_id, _user_id) = extract_identity(token, &state.db)
        .await
        .map_err(|e| (StatusCode::UNAUTHORIZED, e.message().to_string()))?;

    let archive_path = state
        .store
        .resolve(&tenant_id, &archive_id)
        .map_err(|e| (StatusCode::NOT_FOUND, e.message().to_string()))?;

    tracing::info!(tenant_id = %tenant_id, archive_id = %archive_id, "upload: receiving");

    // Phase 1: receive bytes to a temp directory — no lock held yet,
    // so multiple uploads to the same vault can transfer concurrently.
    let tmp =
        tempfile::tempdir().map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let mut files: Vec<(String, PathBuf, u32, u64)> = Vec::new();
    let now_secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?
    {
        let dst_path = field
            .name()
            .map(|s| s.to_string())
            .or_else(|| field.file_name().map(|s| s.to_string()))
            .ok_or((StatusCode::BAD_REQUEST, "field missing name".into()))?;

        // Reject path traversal attempts (e.g. ../../etc/passwd).
        let safe_rel = sanitize_path(&dst_path)
            .ok_or((StatusCode::BAD_REQUEST, "invalid file path".into()))?;
        let on_disk = tmp.path().join(&safe_rel);
        if let Some(par) = on_disk.parent() {
            std::fs::create_dir_all(par)
                .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
        }

        let bytes = field
            .bytes()
            .await
            .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;

        std::fs::write(&on_disk, &bytes)
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

        files.push((dst_path, on_disk, 0o644, now_secs));
    }

    // Phase 2: acquire lock only for the archive mutation (fast, disk-local).
    let lock_key = format!("{tenant_id}:{archive_id}");
    let lock = state
        .archive_locks
        .entry(lock_key)
        .or_insert_with(|| Arc::new(Mutex::new(())))
        .clone();
    let _guard = lock.lock_owned().await;
    tracing::info!(tenant_id = %tenant_id, archive_id = %archive_id, files = files.len(), "upload: committing");

    if files.is_empty() {
        return Err((StatusCode::BAD_REQUEST, "no files in request".into()));
    }

    // Commit to archive on a blocking thread; keep tmp alive for the duration.
    let archive_path_clone = archive_path.clone();
    let result = tokio::task::spawn_blocking(move || -> arx_core::error::Result<()> {
        let mut arc = CrudArchive::open_with_crypto(&archive_path_clone, None, [0u8; 32])?;
        for (dst, src, mode, mtime) in &files {
            arc.put_file(src, dst, *mode, *mtime)?;
        }
        Ok(())
    })
    .await;

    drop(tmp);

    match result {
        Ok(Ok(())) => Ok(Json(json!({ "ok": true }))),
        Ok(Err(e)) => Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string())),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string())),
    }
}
