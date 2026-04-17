use argon2::{
    Argon2, PasswordHasher, PasswordVerifier,
    password_hash::{PasswordHash, SaltString, rand_core::OsRng},
};
use sha2::{Digest, Sha256};
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

fn now_secs() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64
}

pub fn sha256_hex(data: &[u8]) -> String {
    Sha256::digest(data)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

fn random_bytes(count: usize) -> Vec<u8> {
    let mut out = Vec::with_capacity(count);
    let mut remaining = count;
    while remaining > 0 {
        let u = Uuid::new_v4();
        let take = remaining.min(16);
        out.extend_from_slice(&u.as_bytes()[..take]);
        remaining -= take;
    }
    out
}

pub fn random_hex(byte_count: usize) -> String {
    random_bytes(byte_count)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

pub struct TenantInfo {
    pub id: String,
    pub name: String,
    pub created_at: i64,
}

pub struct UserInfo {
    pub id: String,
    pub email: String,
    pub tenant_id: String,
    pub active: bool,
    pub created_at: i64,
}

pub struct AuthDb {
    db: libsql::Database,
}

// Schema — tables created individually so execute_batch is not required.
const TABLES: &[&str] = &[
    "CREATE TABLE IF NOT EXISTS tenants (
        id TEXT PRIMARY KEY,
        name TEXT NOT NULL UNIQUE,
        created_at INTEGER NOT NULL
    )",
    "CREATE TABLE IF NOT EXISTS users (
        id TEXT PRIMARY KEY,
        tenant_id TEXT NOT NULL,
        email TEXT NOT NULL UNIQUE,
        password_hash TEXT NOT NULL,
        active INTEGER NOT NULL DEFAULT 1,
        created_at INTEGER NOT NULL
    )",
    "CREATE TABLE IF NOT EXISTS api_keys (
        key_hash TEXT PRIMARY KEY,
        user_id TEXT NOT NULL,
        tenant_id TEXT NOT NULL,
        name TEXT NOT NULL DEFAULT '',
        created_at INTEGER NOT NULL,
        expires_at INTEGER,
        revoked INTEGER NOT NULL DEFAULT 0
    )",
    "CREATE TABLE IF NOT EXISTS refresh_tokens (
        token_hash TEXT PRIMARY KEY,
        user_id TEXT NOT NULL,
        tenant_id TEXT NOT NULL,
        created_at INTEGER NOT NULL,
        expires_at INTEGER NOT NULL,
        revoked INTEGER NOT NULL DEFAULT 0
    )",
    "CREATE TABLE IF NOT EXISTS jwt_secret (
        id INTEGER PRIMARY KEY,
        secret BLOB NOT NULL,
        created_at INTEGER NOT NULL
    )",
];

impl AuthDb {
    /// Open the auth database.
    ///
    /// When `TURSO_URL` + `TURSO_AUTH_TOKEN` env vars are set the database is
    /// served from Turso's cloud (recommended for production). Otherwise falls
    /// back to a local SQLite file at `path`.
    pub async fn open(path: &Path) -> Result<Self, String> {
        let turso_url = std::env::var("TURSO_URL").ok();
        let turso_token = std::env::var("TURSO_AUTH_TOKEN").ok();

        let db = match (turso_url.as_deref(), turso_token.as_deref()) {
            (Some(url), Some(token)) if !url.is_empty() && !token.is_empty() => {
                tracing::info!("auth DB: connecting to Turso remote database");
                libsql::Builder::new_remote(url.to_string(), token.to_string())
                    .build()
                    .await
                    .map_err(|e| e.to_string())?
            }
            _ => {
                tracing::info!("auth DB: using local SQLite at {path:?}");
                libsql::Builder::new_local(path)
                    .build()
                    .await
                    .map_err(|e| e.to_string())?
            }
        };

        let conn = db.connect().map_err(|e| e.to_string())?;

        for stmt in TABLES {
            conn.execute(stmt, ()).await.map_err(|e| e.to_string())?;
        }

        // Generate JWT secret on first boot
        let mut rows = conn
            .query("SELECT COUNT(*) FROM jwt_secret", ())
            .await
            .map_err(|e| e.to_string())?;
        let count: i64 = rows
            .next()
            .await
            .map_err(|e| e.to_string())?
            .ok_or("no rows from COUNT")?
            .get(0)
            .map_err(|e| e.to_string())?;

        if count == 0 {
            let secret = random_bytes(64);
            conn.execute(
                "INSERT INTO jwt_secret (id, secret, created_at) VALUES (1, ?1, ?2)",
                libsql::params![secret, now_secs()],
            )
            .await
            .map_err(|e| e.to_string())?;
            eprintln!("info: generated new JWT signing secret");
        }

        Ok(Self { db })
    }

    fn conn(&self) -> Result<libsql::Connection, String> {
        self.db.connect().map_err(|e| e.to_string())
    }

    // ── Auth ─────────────────────────────────────────────────────────────────

    /// Validate an API key. Returns (tenant_id, user_id) on success.
    pub async fn authenticate_key(
        &self,
        raw_key: &str,
    ) -> Result<Option<(String, String)>, String> {
        let key_hash = sha256_hex(raw_key.as_bytes());
        let now = now_secs();
        let conn = self.conn()?;
        let mut rows = conn
            .query(
                "SELECT tenant_id, user_id FROM api_keys \
                 WHERE key_hash = ?1 AND revoked = 0 \
                 AND (expires_at IS NULL OR expires_at > ?2)",
                libsql::params![key_hash, now],
            )
            .await
            .map_err(|e| e.to_string())?;

        match rows.next().await.map_err(|e| e.to_string())? {
            None => Ok(None),
            Some(row) => {
                let tenant_id: String = row.get(0).map_err(|e| e.to_string())?;
                let user_id: String = row.get(1).map_err(|e| e.to_string())?;
                Ok(Some((tenant_id, user_id)))
            }
        }
    }

    /// Retrieve the HMAC secret used to sign/verify JWTs.
    pub async fn get_jwt_secret(&self) -> Result<Vec<u8>, String> {
        let conn = self.conn()?;
        let mut rows = conn
            .query("SELECT secret FROM jwt_secret WHERE id = 1", ())
            .await
            .map_err(|e| e.to_string())?;
        let row = rows
            .next()
            .await
            .map_err(|e| e.to_string())?
            .ok_or_else(|| "jwt_secret row missing".to_string())?;
        row.get(0).map_err(|e| e.to_string())
    }

    /// Verify email + password. Returns (user_id, tenant_id) if valid.
    pub async fn verify_login(
        &self,
        email: &str,
        password: &str,
    ) -> Result<Option<(String, String)>, String> {
        let conn = self.conn()?;
        let mut rows = conn
            .query(
                "SELECT id, tenant_id, password_hash FROM users \
                 WHERE email = ?1 AND active = 1",
                libsql::params![email.to_string()],
            )
            .await
            .map_err(|e| e.to_string())?;

        let row = match rows.next().await.map_err(|e| e.to_string())? {
            None => return Ok(None),
            Some(r) => r,
        };

        let user_id: String = row.get(0).map_err(|e| e.to_string())?;
        let tenant_id: String = row.get(1).map_err(|e| e.to_string())?;
        let hash: String = row.get(2).map_err(|e| e.to_string())?;

        let parsed = PasswordHash::new(&hash).map_err(|e| e.to_string())?;
        match Argon2::default().verify_password(password.as_bytes(), &parsed) {
            Ok(()) => Ok(Some((user_id, tenant_id))),
            Err(_) => Ok(None),
        }
    }

    // ── Refresh tokens ────────────────────────────────────────────────────────

    pub async fn create_refresh_token(
        &self,
        user_id: &str,
        tenant_id: &str,
    ) -> Result<String, String> {
        let raw = random_hex(32);
        let hash = sha256_hex(raw.as_bytes());
        let expires_at = now_secs() + 30 * 24 * 3600;
        let conn = self.conn()?;
        conn.execute(
            "INSERT INTO refresh_tokens \
             (token_hash, user_id, tenant_id, created_at, expires_at, revoked) \
             VALUES (?1, ?2, ?3, ?4, ?5, 0)",
            libsql::params![
                hash,
                user_id.to_string(),
                tenant_id.to_string(),
                now_secs(),
                expires_at
            ],
        )
        .await
        .map_err(|e| e.to_string())?;
        Ok(raw)
    }

    /// Verify + revoke the given refresh token (rotation). Returns (user_id, tenant_id).
    pub async fn consume_refresh_token(
        &self,
        raw_token: &str,
    ) -> Result<Option<(String, String)>, String> {
        let hash = sha256_hex(raw_token.as_bytes());
        let now = now_secs();
        let conn = self.conn()?;

        let mut rows = conn
            .query(
                "SELECT user_id, tenant_id FROM refresh_tokens \
                 WHERE token_hash = ?1 AND revoked = 0 AND expires_at > ?2",
                libsql::params![hash.clone(), now],
            )
            .await
            .map_err(|e| e.to_string())?;

        let row = match rows.next().await.map_err(|e| e.to_string())? {
            None => return Ok(None),
            Some(r) => r,
        };

        let user_id: String = row.get(0).map_err(|e| e.to_string())?;
        let tenant_id: String = row.get(1).map_err(|e| e.to_string())?;

        // Revoke on use (token rotation)
        conn.execute(
            "UPDATE refresh_tokens SET revoked = 1 WHERE token_hash = ?1",
            libsql::params![hash],
        )
        .await
        .map_err(|e| e.to_string())?;

        Ok(Some((user_id, tenant_id)))
    }

    pub async fn revoke_refresh_token(&self, raw_token: &str) -> Result<(), String> {
        let hash = sha256_hex(raw_token.as_bytes());
        let conn = self.conn()?;
        conn.execute(
            "UPDATE refresh_tokens SET revoked = 1 WHERE token_hash = ?1",
            libsql::params![hash],
        )
        .await
        .map_err(|e| e.to_string())?;

        Ok(())
    }

    // ── User lookup ───────────────────────────────────────────────────────────

    pub async fn get_user_info(&self, user_id: &str) -> Result<Option<(String, String)>, String> {
        let conn = self.conn()?;
        let mut rows = conn
            .query(
                "SELECT email, tenant_id FROM users WHERE id = ?1",
                libsql::params![user_id.to_string()],
            )
            .await
            .map_err(|e| e.to_string())?;

        match rows.next().await.map_err(|e| e.to_string())? {
            None => Ok(None),
            Some(row) => {
                let email: String = row.get(0).map_err(|e| e.to_string())?;
                let tenant_id: String = row.get(1).map_err(|e| e.to_string())?;
                Ok(Some((email, tenant_id)))
            }
        }
    }

    // ── Tenant management ────────────────────────────────────────────────────

    pub async fn create_tenant(&self, name: &str) -> Result<String, String> {
        let id = Uuid::new_v4().to_string();
        self.conn()?
            .execute(
                "INSERT INTO tenants (id, name, created_at) VALUES (?1, ?2, ?3)",
                libsql::params![id.clone(), name.to_string(), now_secs()],
            )
            .await
            .map_err(|e| e.to_string())?;
        Ok(id)
    }

    pub async fn list_tenants(&self) -> Result<Vec<TenantInfo>, String> {
        let conn = self.conn()?;
        let mut rows = conn
            .query(
                "SELECT id, name, created_at FROM tenants ORDER BY created_at",
                (),
            )
            .await
            .map_err(|e| e.to_string())?;
        let mut result = Vec::new();
        while let Some(row) = rows.next().await.map_err(|e| e.to_string())? {
            result.push(TenantInfo {
                id: row.get(0).map_err(|e| e.to_string())?,
                name: row.get(1).map_err(|e| e.to_string())?,
                created_at: row.get(2).map_err(|e| e.to_string())?,
            });
        }
        Ok(result)
    }

    // ── User management ───────────────────────────────────────────────────────

    pub async fn create_user(
        &self,
        tenant_id: &str,
        email: &str,
        password: &str,
    ) -> Result<String, String> {
        let id = Uuid::new_v4().to_string();
        let salt = SaltString::generate(&mut OsRng);
        let hash = Argon2::default()
            .hash_password(password.as_bytes(), &salt)
            .map_err(|e| e.to_string())?
            .to_string();
        self.conn()?
            .execute(
                "INSERT INTO users (id, tenant_id, email, password_hash, active, created_at) \
                 VALUES (?1, ?2, ?3, ?4, 1, ?5)",
                libsql::params![
                    id.clone(),
                    tenant_id.to_string(),
                    email.to_string(),
                    hash,
                    now_secs()
                ],
            )
            .await
            .map_err(|e| e.to_string())?;
        Ok(id)
    }

    pub async fn list_users(&self, tenant_id: &str) -> Result<Vec<UserInfo>, String> {
        let conn = self.conn()?;
        let mut rows = conn
            .query(
                "SELECT id, email, tenant_id, active, created_at FROM users \
                 WHERE tenant_id = ?1 ORDER BY created_at",
                libsql::params![tenant_id.to_string()],
            )
            .await
            .map_err(|e| e.to_string())?;
        let mut result = Vec::new();
        while let Some(row) = rows.next().await.map_err(|e| e.to_string())? {
            let active: i64 = row.get(3).map_err(|e| e.to_string())?;
            result.push(UserInfo {
                id: row.get(0).map_err(|e| e.to_string())?,
                email: row.get(1).map_err(|e| e.to_string())?,
                tenant_id: row.get(2).map_err(|e| e.to_string())?,
                active: active != 0,
                created_at: row.get(4).map_err(|e| e.to_string())?,
            });
        }
        Ok(result)
    }

    // ── API key management ────────────────────────────────────────────────────

    /// Create an API key. Returns the raw hex key (shown once).
    pub async fn create_api_key(
        &self,
        user_id: &str,
        tenant_id: &str,
        name: &str,
        expires_at: Option<i64>,
    ) -> Result<String, String> {
        let raw = random_hex(32);
        let key_hash = sha256_hex(raw.as_bytes());
        let conn = self.conn()?;
        match expires_at {
            None => {
                conn.execute(
                    "INSERT INTO api_keys \
                     (key_hash, user_id, tenant_id, name, created_at, revoked) \
                     VALUES (?1, ?2, ?3, ?4, ?5, 0)",
                    libsql::params![
                        key_hash,
                        user_id.to_string(),
                        tenant_id.to_string(),
                        name.to_string(),
                        now_secs()
                    ],
                )
                .await
                .map_err(|e| e.to_string())?;
            }
            Some(exp) => {
                conn.execute(
                    "INSERT INTO api_keys \
                     (key_hash, user_id, tenant_id, name, created_at, expires_at, revoked) \
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, 0)",
                    libsql::params![
                        key_hash,
                        user_id.to_string(),
                        tenant_id.to_string(),
                        name.to_string(),
                        now_secs(),
                        exp
                    ],
                )
                .await
                .map_err(|e| e.to_string())?;
            }
        }
        Ok(raw)
    }

    pub async fn revoke_api_key(&self, key_hash: &str) -> Result<(), String> {
        self.conn()?
            .execute(
                "UPDATE api_keys SET revoked = 1 WHERE key_hash = ?1",
                libsql::params![key_hash.to_string()],
            )
            .await
            .map(|_| ())
            .map_err(|e| e.to_string())
    }
}
