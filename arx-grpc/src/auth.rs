use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

/// Maps API key hex strings to tenant UUIDs.
#[derive(Serialize, Deserialize, Default, Clone)]
pub struct TenantStore {
    /// Map from API-key (32-byte hex) to tenant UUID string.
    pub api_keys: HashMap<String, String>,
}

impl TenantStore {
    /// Load from a JSON file. Returns an empty store if the file doesn't exist.
    pub fn load(path: &Path) -> Self {
        if let Ok(data) = std::fs::read_to_string(path) {
            serde_json::from_str(&data).unwrap_or_default()
        } else {
            eprintln!("warn: tenants.json not found at {} — all requests will be rejected", path.display());
            Self::default()
        }
    }

    /// Validate an API key. Returns the tenant_id if valid.
    pub fn authenticate(&self, bearer: &str) -> Option<&str> {
        // Strip "Bearer " prefix if present
        let key = bearer.strip_prefix("Bearer ").unwrap_or(bearer).trim();
        self.api_keys.get(key).map(|s| s.as_str())
    }
}

/// Extract the tenant ID from a tonic request's metadata.
/// Looks for `authorization: Bearer <api_key>` header.
pub fn extract_tenant<T>(
    req: &tonic::Request<T>,
    store: &TenantStore,
) -> Result<String, tonic::Status> {
    let auth = req
        .metadata()
        .get("authorization")
        .ok_or_else(|| tonic::Status::unauthenticated("missing authorization header"))?
        .to_str()
        .map_err(|_| tonic::Status::unauthenticated("invalid authorization header"))?;

    store
        .authenticate(auth)
        .map(|t| t.to_string())
        .ok_or_else(|| tonic::Status::unauthenticated("invalid API key"))
}
