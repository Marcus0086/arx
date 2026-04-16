use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

/// Resolved file paths for a single archive.
pub struct ArchivePaths {
    pub dir: PathBuf,
    pub data: PathBuf,   // data.arx
    pub journal: PathBuf, // data.arx.log
    pub delta: PathBuf,  // data.arx.delta
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
            dir,
        }
    }

    /// Allocate a new archive directory and return its UUID.
    pub fn new_archive(
        &self,
        tenant_id: &str,
        _name: &str,
    ) -> std::io::Result<String> {
        let id = Uuid::new_v4().to_string();
        let dir = self.archive_dir(tenant_id, &id);
        fs::create_dir_all(&dir)?;
        Ok(id)
    }

    /// Resolve archive ownership: returns the data.arx path if it exists and belongs
    /// to the tenant, otherwise returns an error.
    pub fn resolve(
        &self,
        tenant_id: &str,
        archive_id: &str,
    ) -> Result<PathBuf, tonic::Status> {
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

    /// List all archives for a tenant.
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
                let size = fs::metadata(&data_path).map(|m| m.len()).unwrap_or(0);
                let created_at = fs::metadata(&data_path)
                    .and_then(|m| m.created())
                    .unwrap_or(SystemTime::UNIX_EPOCH)
                    .duration_since(UNIX_EPOCH)
                    .map(|d| d.as_secs().to_string())
                    .unwrap_or_default();

                Some(crate::arx::ArchiveInfo {
                    id: archive_id.clone(),
                    name: archive_id,
                    size_bytes: size,
                    created_at,
                    encrypted: false, // TODO: read flag from superblock
                })
            })
            .collect()
    }

    /// Delete an archive and all its sidecars.
    pub fn delete_archive(
        &self,
        tenant_id: &str,
        archive_id: &str,
    ) -> Result<(), tonic::Status> {
        if archive_id.contains('/') || archive_id.contains("..") {
            return Err(tonic::Status::invalid_argument("invalid archive_id"));
        }
        let dir = self.archive_dir(tenant_id, archive_id);
        if !dir.exists() {
            return Err(tonic::Status::not_found(format!(
                "archive {archive_id} not found"
            )));
        }
        fs::remove_dir_all(&dir)
            .map_err(|e| tonic::Status::internal(e.to_string()))
    }
}
