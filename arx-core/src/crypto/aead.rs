use blake3::Hasher;
use chacha20poly1305::{
    Key, XChaCha20Poly1305, XNonce,
    aead::{Aead, KeyInit},
};

use crate::error::{ArxError, Result};

pub const TAG_LEN: usize = 16;

/// Raw 32-byte encryption key.
#[derive(Clone)]
pub struct AeadKey(pub [u8; 32]);

/// Region IDs for domain separation — each region gets a unique nonce.
pub enum Region {
    Manifest = 1,
    ChunkTable = 2,
    ChunkData = 3, // per-chunk: chunk_id included in nonce
}

/// Derive a 24-byte XChaCha20 nonce.
/// nonce = blake3(key_salt || region_byte || chunk_id_le).take(24)
pub fn derive_nonce(key_salt: &[u8; 32], region: Region, chunk_id: u64) -> XNonce {
    let mut h = Hasher::new();
    h.update(key_salt);
    h.update(&[region as u8]);
    h.update(&chunk_id.to_le_bytes());
    let out = h.finalize();
    XNonce::from_slice(&out.as_bytes()[..24]).to_owned()
}

/// Encrypt `plaintext` with XChaCha20-Poly1305. Returns ciphertext + 16-byte tag.
/// Panics only if the cipher itself fails (never happens with valid key/nonce).
pub fn seal_whole(key: &AeadKey, nonce: &XNonce, ad: &[u8], plaintext: &[u8]) -> Vec<u8> {
    let aead = XChaCha20Poly1305::new(Key::from_slice(&key.0));
    aead.encrypt(
        nonce,
        chacha20poly1305::aead::Payload {
            msg: plaintext,
            aad: ad,
        },
    )
    .expect("XChaCha20 encrypt should never fail for valid key/nonce")
}

/// Decrypt and authenticate `ciphertext`. Returns `Err(AeadError)` if the tag
/// does not match — indicating a wrong key or tampered ciphertext.
pub fn open_whole(key: &AeadKey, nonce: &XNonce, ad: &[u8], ciphertext: &[u8]) -> Result<Vec<u8>> {
    let aead = XChaCha20Poly1305::new(Key::from_slice(&key.0));
    aead.decrypt(
        nonce,
        chacha20poly1305::aead::Payload {
            msg: ciphertext,
            aad: ad,
        },
    )
    .map_err(|_| ArxError::AeadError)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_key() -> AeadKey {
        AeadKey([0x42u8; 32])
    }
    fn test_salt() -> [u8; 32] {
        [0x11u8; 32]
    }

    #[test]
    fn test_seal_open_roundtrip() {
        let key = test_key();
        let nonce = derive_nonce(&test_salt(), Region::Manifest, 0);
        let plaintext = b"hello arx";
        let ct = seal_whole(&key, &nonce, b"manifest", plaintext);
        let pt = open_whole(&key, &nonce, b"manifest", &ct).unwrap();
        assert_eq!(pt, plaintext);
    }

    #[test]
    fn test_wrong_key_returns_aead_error() {
        let key = test_key();
        let wrong_key = AeadKey([0xFFu8; 32]);
        let nonce = derive_nonce(&test_salt(), Region::ChunkData, 7);
        let ct = seal_whole(&key, &nonce, b"chunk", b"secret data");
        let err = open_whole(&wrong_key, &nonce, b"chunk", &ct).unwrap_err();
        assert!(matches!(err, ArxError::AeadError));
    }

    #[test]
    fn test_tampered_ciphertext_returns_aead_error() {
        let key = test_key();
        let nonce = derive_nonce(&test_salt(), Region::ChunkTable, 0);
        let mut ct = seal_whole(&key, &nonce, b"chunktab", b"table data");
        ct[0] ^= 0x01; // flip one bit
        let err = open_whole(&key, &nonce, b"chunktab", &ct).unwrap_err();
        assert!(matches!(err, ArxError::AeadError));
    }

    #[test]
    fn test_nonce_determinism() {
        let salt = test_salt();
        let n1 = derive_nonce(&salt, Region::ChunkData, 42);
        let n2 = derive_nonce(&salt, Region::ChunkData, 42);
        assert_eq!(n1, n2);
    }

    #[test]
    fn test_different_regions_different_nonces() {
        let salt = test_salt();
        let n_manifest = derive_nonce(&salt, Region::Manifest, 0);
        let n_table = derive_nonce(&salt, Region::ChunkTable, 0);
        assert_ne!(n_manifest, n_table);
    }
}
