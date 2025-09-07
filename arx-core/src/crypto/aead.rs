use blake3::Hasher;
use chacha20poly1305::{
    Key, XChaCha20Poly1305, XNonce,
    aead::{Aead, KeyInit},
};

pub const TAG_LEN: usize = 16;

/// Keys: for alpha we support raw 32-byte keys.
#[derive(Clone)]
pub struct AeadKey(pub [u8; 32]);

/// Static region IDs (domain separation)
pub enum Region {
    Manifest = 1,
    ChunkTable = 2,
    ChunkData = 3, // per-chunk ⇒ add chunk_id in nonce derivation
}

/// Nonce derivation: XChaCha requires 24-byte nonce.
/// nonce = blake3(key || salt || region || chunk_id).take(24)
pub fn derive_nonce(key_salt: &[u8; 32], region: Region, chunk_id: u64) -> XNonce {
    let mut h = Hasher::new();
    h.update(key_salt);
    h.update(&[region as u8]);
    h.update(&chunk_id.to_le_bytes());
    let out = h.finalize(); // 32 bytes
    XNonce::from_slice(&out.as_bytes()[..24]).to_owned()
}

/// Seal a whole buffer (associated data optional).
pub fn seal_whole(key: &AeadKey, nonce: &XNonce, ad: &[u8], plaintext: &[u8]) -> Vec<u8> {
    let aead = XChaCha20Poly1305::new(Key::from_slice(&key.0));
    aead.encrypt(
        nonce,
        chacha20poly1305::aead::Payload {
            msg: plaintext,
            aad: ad,
        },
    )
    .expect("encrypt")
}

/// Open a whole buffer.
pub fn open_whole(key: &AeadKey, nonce: &XNonce, ad: &[u8], ciphertext: &[u8]) -> Vec<u8> {
    let aead = XChaCha20Poly1305::new(Key::from_slice(&key.0));
    aead.decrypt(
        nonce,
        chacha20poly1305::aead::Payload {
            msg: ciphertext,
            aad: ad,
        },
    )
    .expect("decrypt")
}
