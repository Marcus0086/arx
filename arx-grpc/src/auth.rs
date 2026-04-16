use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey, Header, Validation, decode, encode};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::db::AuthDb;

#[derive(Debug, Serialize, Deserialize)]
pub struct JwtClaims {
    pub sub: String,  // user_id
    pub tid: String,  // tenant_id
    pub exp: usize,
    pub iat: usize,
}

fn now_secs() -> usize {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as usize
}

/// Synchronously extract the Bearer token from request metadata (returns owned String).
/// No borrow is held across any await point, so T does not need to be Sync.
pub fn extract_bearer<T>(req: &tonic::Request<T>) -> Result<String, tonic::Status> {
    req.metadata()
        .get("authorization")
        .ok_or_else(|| tonic::Status::unauthenticated("missing authorization header"))?
        .to_str()
        .map(|s| s.strip_prefix("Bearer ").unwrap_or(s).trim().to_string())
        .map_err(|_| tonic::Status::unauthenticated("invalid authorization header"))
}

/// Async: resolve tenant_id from a bearer token string.
pub async fn extract_tenant(token: &str, db: &AuthDb) -> Result<String, tonic::Status> {
    let (tenant_id, _) = extract_identity(token, db).await?;
    Ok(tenant_id)
}

/// Async: resolve (tenant_id, user_id) from a bearer token string.
pub async fn extract_identity(
    token: &str,
    db: &AuthDb,
) -> Result<(String, String), tonic::Status> {
    if token.contains('.') {
        let secret = db
            .get_jwt_secret()
            .await
            .map_err(|e| tonic::Status::internal(e))?;
        let claims = verify_jwt(token, &secret)?;
        Ok((claims.tid, claims.sub))
    } else {
        db.authenticate_key(token)
            .await
            .map_err(|e| tonic::Status::internal(e))?
            .ok_or_else(|| tonic::Status::unauthenticated("invalid API key"))
    }
}

/// Verify that the request carries the admin key (ARX_ADMIN_KEY env var). Fully sync.
pub fn check_admin<T>(req: &tonic::Request<T>, admin_key: &str) -> Result<(), tonic::Status> {
    if admin_key.is_empty() {
        return Err(tonic::Status::unauthenticated(
            "admin RPCs are disabled — set ARX_ADMIN_KEY environment variable",
        ));
    }
    let provided = extract_bearer(req).unwrap_or_default();
    // Compare SHA-256 digests to prevent naive timing leaks.
    let expected = Sha256::digest(admin_key.as_bytes());
    let actual = Sha256::digest(provided.as_bytes());
    if expected == actual {
        Ok(())
    } else {
        Err(tonic::Status::unauthenticated("invalid admin key"))
    }
}

/// Issue a 15-minute HS256 JWT access token.
pub fn issue_jwt(user_id: &str, tenant_id: &str, secret: &[u8]) -> Result<String, tonic::Status> {
    let now = now_secs();
    let claims = JwtClaims {
        sub: user_id.to_string(),
        tid: tenant_id.to_string(),
        exp: now + 900,
        iat: now,
    };
    encode(
        &Header::new(Algorithm::HS256),
        &claims,
        &EncodingKey::from_secret(secret),
    )
    .map_err(|e| tonic::Status::internal(format!("jwt encode: {e}")))
}

/// Verify an HS256 JWT and return its claims.
pub fn verify_jwt(token: &str, secret: &[u8]) -> Result<JwtClaims, tonic::Status> {
    decode::<JwtClaims>(
        token,
        &DecodingKey::from_secret(secret),
        &Validation::new(Algorithm::HS256),
    )
    .map(|d| d.claims)
    .map_err(|e| tonic::Status::unauthenticated(format!("invalid JWT: {e}")))
}
