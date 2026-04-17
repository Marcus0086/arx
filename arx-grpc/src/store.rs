use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

/// Map arx-core's in-memory Stats to the proto ArchiveStats message.
pub fn into_proto_stats(s: arx_core::stats::Stats) -> crate::arx::ArchiveStats {
    let stored = s.physical_bytes_base + s.physical_bytes_delta;
    let savings = s.logical_bytes.saturating_sub(stored);
    crate::arx::ArchiveStats {
        files: s.files,
        dirs: s.dirs,
        chunks: s.chunks,
        logical_bytes: s.logical_bytes,
        stored_bytes: stored,
        compression_ratio: s.compression_ratio,
        savings_bytes: savings,
    }
}

/// Resolved file paths for a single archive.
pub struct ArchivePaths {
    pub dir: PathBuf,
    pub data: PathBuf,    // data.arx
    pub journal: PathBuf, // data.arx.log
    pub delta: PathBuf,   // data.arx.delta
    pub meta: PathBuf,    // meta.json (sidecar for mutable metadata)
}

/// Mutable per-archive metadata stored in meta.json alongside data.arx.
/// The CBOR manifest inside data.arx is sealed at pack time and can't be
/// updated without rewriting the whole archive, so mutable fields like the
/// user-facing label live here.
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct ArchiveMeta {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    #[serde(default)]
    pub updated_at: u64,
}

/// Manages the on-disk storage layout for all tenants.
///
/// Layout:
/// ```
/// {root}/tenants/{tenant_id}/archives/{archive_id}/data.arx
///                                                  data.arx.log
///                                                  data.arx.delta
///                                                  meta.json
/// ```
#[derive(Clone)]
pub struct ArchiveStore {
    pub root: PathBuf,
}

impl ArchiveStore {
    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }

    fn tenant_dir(&self, tenant_id: &str) -> PathBuf {
        self.root.join("tenants").join(tenant_id)
    }

    fn archives_dir(&self, tenant_id: &str) -> PathBuf {
        self.tenant_dir(tenant_id).join("archives")
    }

    fn archive_dir(&self, tenant_id: &str, archive_id: &str) -> PathBuf {
        self.archives_dir(tenant_id).join(archive_id)
    }

    pub fn archive_paths(&self, tenant_id: &str, archive_id: &str) -> ArchivePaths {
        let dir = self.archive_dir(tenant_id, archive_id);
        ArchivePaths {
            data: dir.join("data.arx"),
            journal: dir.join("data.arx.log"),
            delta: dir.join("data.arx.delta"),
            meta: dir.join("meta.json"),
            dir,
        }
    }

    /// Read the mutable metadata sidecar for an archive. Returns None if no
    /// sidecar exists or it fails to parse.
    pub fn read_meta(&self, tenant_id: &str, archive_id: &str) -> Option<ArchiveMeta> {
        let path = self.archive_paths(tenant_id, archive_id).meta;
        let bytes = fs::read(&path).ok()?;
        serde_json::from_slice::<ArchiveMeta>(&bytes).ok()
    }

    /// Read just the user-facing label from the meta sidecar.
    pub fn read_label(&self, tenant_id: &str, archive_id: &str) -> Option<String> {
        self.read_meta(tenant_id, archive_id).and_then(|m| m.label)
    }

    /// Write the user-facing label to the meta sidecar. Creates the file if
    /// missing; preserves other fields if present.
    pub fn write_label(
        &self,
        tenant_id: &str,
        archive_id: &str,
        label: &str,
    ) -> std::io::Result<()> {
        let mut meta = self.read_meta(tenant_id, archive_id).unwrap_or_default();
        meta.label = Some(label.to_string());
        meta.updated_at = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let path = self.archive_paths(tenant_id, archive_id).meta;
        let tmp = path.with_extension("json.tmp");
        let json = serde_json::to_vec_pretty(&meta)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        fs::write(&tmp, &json)?;
        fs::rename(&tmp, &path)?;
        Ok(())
    }

    /// Allocate a new archive directory and return its UUID.
    pub fn new_archive(&self, tenant_id: &str, _name: &str) -> std::io::Result<String> {
        let id = Uuid::new_v4().to_string();
        let dir = self.archive_dir(tenant_id, &id);
        fs::create_dir_all(&dir)?;
        Ok(id)
    }

    /// Resolve archive ownership: returns the data.arx path if it exists and belongs
    /// to the tenant, otherwise returns an error.
    pub fn resolve(&self, tenant_id: &str, archive_id: &str) -> Result<PathBuf, tonic::Status> {
        // Prevent path traversal
        if archive_id.contains('/') || archive_id.contains("..") {
            return Err(tonic::Status::invalid_argument("invalid archive_id"));
        }
        let paths = self.archive_paths(tenant_id, archive_id);
        if !paths.data.exists() {
            return Err(tonic::Status::not_found(format!(
                "archive {archive_id} not found"
            )));
        }
        Ok(paths.data)
    }

    /// List all archives for a tenant, computing stats + resolving labels from
    /// the meta.json sidecar (falling back to the manifest label, then to a UUID
    /// prefix).
    pub fn list_archives(&self, tenant_id: &str) -> Vec<crate::arx::ArchiveInfo> {
        let archives_dir = self.archives_dir(tenant_id);
        let Ok(entries) = fs::read_dir(&archives_dir) else {
            return vec![];
        };

        entries
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().map(|t| t.is_dir()).unwrap_or(false))
            .filter_map(|e| {
                let archive_id = e.file_name().to_string_lossy().to_string();
                let data_path = e.path().join("data.arx");
                if !data_path.exists() {
                    return None;
                }

                let created_at = fs::metadata(&data_path)
                    .and_then(|m| m.created())
                    .unwrap_or(SystemTime::UNIX_EPOCH)
                    .duration_since(UNIX_EPOCH)
                    .map(|d| d.as_secs().to_string())
                    .unwrap_or_default();

                // Stats (cheap: reads superblock + manifest, not data chunks).
                let stats = arx_core::read::stats::compute_stats(&data_path, None).ok();
                let encrypted =
                    arx_core::read::stats::read_encrypted_flag(&data_path).unwrap_or(false);

                // Label resolution: meta.json > manifest meta.label > UUID prefix.
                let name = self
                    .read_label(tenant_id, &archive_id)
                    .or_else(|| {
                        if !encrypted {
                            arx_core::read::stats::read_manifest_label(&data_path, None)
                        } else {
                            None
                        }
                    })
                    .unwrap_or_else(|| archive_id.chars().take(8).collect::<String>());

                let size_bytes = stats
                    .as_ref()
                    .map(|s| s.physical_bytes_base + s.physical_bytes_delta)
                    .unwrap_or_else(|| fs::metadata(&data_path).map(|m| m.len()).unwrap_or(0));

                Some(crate::arx::ArchiveInfo {
                    id: archive_id,
                    name,
                    size_bytes,
                    created_at,
                    encrypted,
                    stats: stats.map(into_proto_stats),
                })
            })
            .collect()
    }

    /// Delete an archive and all its sidecars.
    pub fn delete_archive(&self, tenant_id: &str, archive_id: &str) -> Result<(), tonic::Status> {
        if archive_id.contains('/') || archive_id.contains("..") {
            return Err(tonic::Status::invalid_argument("invalid archive_id"));
        }
        let dir = self.archive_dir(tenant_id, archive_id);
        if !dir.exists() {
            return Err(tonic::Status::not_found(format!(
                "archive {archive_id} not found"
            )));
        }
        fs::remove_dir_all(&dir).map_err(|e| tonic::Status::internal(e.to_string()))
    }
}
