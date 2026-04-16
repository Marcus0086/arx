use argon2::{Algorithm, Argon2, Params, Version};

/// Derive a 32-byte encryption key from a password and a per-archive random salt.
///
/// Parameters: Argon2id, m=65536 (64 MiB), t=3 iterations, p=4 lanes.
/// These are conservative interactive-login parameters — fast enough for CLI
/// use (< 1 s on modern hardware) while resisting GPU brute-force.
pub fn derive_key(password: &str, salt: &[u8; 32]) -> [u8; 32] {
    let params = Params::new(65_536, 3, 4, Some(32))
        .expect("Argon2 params are valid");
    let argon2 = Argon2::new(Algorithm::Argon2id, Version::V0x13, params);
    let mut out = [0u8; 32];
    argon2
        .hash_password_into(password.as_bytes(), salt, &mut out)
        .expect("Argon2 hash_password_into should not fail for valid params");
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_derive_key_determinism() {
        let salt = [0x55u8; 32];
        let k1 = derive_key("hunter2", &salt);
        let k2 = derive_key("hunter2", &salt);
        assert_eq!(k1, k2);
    }

    #[test]
    fn test_different_password_different_key() {
        let salt = [0x55u8; 32];
        let k1 = derive_key("password1", &salt);
        let k2 = derive_key("password2", &salt);
        assert_ne!(k1, k2);
    }

    #[test]
    fn test_different_salt_different_key() {
        let k1 = derive_key("same", &[0x01u8; 32]);
        let k2 = derive_key("same", &[0x02u8; 32]);
        assert_ne!(k1, k2);
    }
}
