/// Chunk integrity tests: verify that per-chunk blake3 mismatch and AEAD
/// tag failures are detected during extraction and verification.
use arx_core::read::extract::{ExtractOptions, extract, verify};
use arx_core::{PackOptions, pack};
use std::fs::{self, OpenOptions};
use std::io::{Seek, SeekFrom, Write};
use tempfile::TempDir;

fn corrupt_byte_in_data_region(archive_path: &std::path::Path) {
    use arx_core::container::superblock::Superblock;
    use std::io::Read;

    // First read the superblock (read-only pass)
    let sb = {
        let mut f = fs::File::open(archive_path).unwrap();
        Superblock::read_from(&mut f).unwrap()
    };

    // Flip a byte somewhere inside the data region (read+write)
    let flip_offset = sb.data_off + 4;
    let mut f = OpenOptions::new()
        .read(true)
        .write(true)
        .open(archive_path)
        .unwrap();
    f.seek(SeekFrom::Start(flip_offset)).unwrap();
    let mut buf = [0u8; 1];
    f.read_exact(&mut buf).unwrap();
    buf[0] ^= 0xFF;
    f.seek(SeekFrom::Start(flip_offset)).unwrap();
    f.write_all(&buf).unwrap();
}

#[test]
fn test_corrupted_plaintext_detected_on_extract() {
    let src = TempDir::new().unwrap();
    let tmp = TempDir::new().unwrap();
    let dst = TempDir::new().unwrap();

    // Write a file large enough to have at least some chunk data
    fs::write(src.path().join("data.bin"), &vec![0xAAu8; 1024]).unwrap();
    let archive = tmp.path().join("corrupt.arx");

    pack(&[src.path()], &archive, None).unwrap();
    corrupt_byte_in_data_region(&archive);

    let result = extract(&archive, dst.path(), None);
    // Either AEAD (if encrypted) or blake3 mismatch should fail
    // For plaintext: blake3 mismatch
    assert!(
        result.is_err(),
        "extraction of corrupted archive should fail, but it succeeded"
    );
}

#[test]
fn test_corrupted_encrypted_chunk_fails_on_aead() {
    let src = TempDir::new().unwrap();
    let tmp = TempDir::new().unwrap();
    let dst = TempDir::new().unwrap();

    fs::write(src.path().join("secret.bin"), &vec![0xBBu8; 2048]).unwrap();
    let archive = tmp.path().join("enc_corrupt.arx");
    let key = [0xDEu8; 32];

    pack(
        &[src.path()],
        &archive,
        Some(&PackOptions {
            aead_key: Some(key),
            ..Default::default()
        }),
    )
    .unwrap();
    corrupt_byte_in_data_region(&archive);

    let ext_opts = ExtractOptions {
        aead_key: Some(key),
        ..Default::default()
    };
    let result = extract(&archive, dst.path(), Some(&ext_opts));
    assert!(
        result.is_err(),
        "extracting tampered encrypted chunk should fail"
    );
}

#[test]
fn test_verify_detects_corruption() {
    let src = TempDir::new().unwrap();
    let tmp = TempDir::new().unwrap();

    fs::write(src.path().join("data.bin"), &vec![0xCCu8; 4096]).unwrap();
    let archive = tmp.path().join("verify_corrupt.arx");
    pack(&[src.path()], &archive, None).unwrap();

    // Verify on clean archive should pass
    verify(&archive, None).expect("verify on clean archive should pass");

    // Corrupt and re-verify
    corrupt_byte_in_data_region(&archive);
    let result = verify(&archive, None);
    assert!(result.is_err(), "verify should fail on corrupted archive");
}
