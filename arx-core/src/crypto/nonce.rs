/// Generate a cryptographically random 32-byte salt using the OS CSPRNG.
pub fn random_salt() -> [u8; 32] {
    let mut salt = [0u8; 32];
    getrandom::getrandom(&mut salt).expect("OS CSPRNG unavailable");
    salt
}
