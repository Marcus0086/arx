use arx_core::read::extract::ExtractOptions;
/// Round-trip integration tests: pack → extract → compare content.
use arx_core::{PackOptions, extract, pack};
use std::fs;
use std::path::Path;
use tempfile::TempDir;

fn write_tree(root: &Path) {
    fs::create_dir_all(root.join("sub")).unwrap();
    fs::write(root.join("hello.txt"), b"hello world\n").unwrap();
    fs::write(root.join("sub/data.bin"), &[0xFFu8; 4096]).unwrap();
    fs::write(root.join("sub/empty.txt"), b"").unwrap();
    // Large file that will span multiple CDC chunks
    let big: Vec<u8> = (0u32..200_000).flat_map(|i| i.to_le_bytes()).collect();
    fs::write(root.join("big.bin"), &big).unwrap();
}

fn compare_trees(src: &Path, dst: &Path) {
    fn collect(base: &Path) -> Vec<(String, Vec<u8>)> {
        let mut out = Vec::new();
        for entry in walkdir::WalkDir::new(base).sort_by_file_name() {
            let entry = entry.unwrap();
            if entry.file_type().is_file() {
                let rel = entry
                    .path()
                    .strip_prefix(base)
                    .unwrap()
                    .to_string_lossy()
                    .into_owned();
                let content = fs::read(entry.path()).unwrap();
                out.push((rel, content));
            }
        }
        out
    }
    let src_files = collect(src);
    let dst_files = collect(dst);
    assert_eq!(src_files.len(), dst_files.len(), "file count mismatch");
    for ((sr, sc), (dr, dc)) in src_files.iter().zip(dst_files.iter()) {
        assert_eq!(sr, dr, "path mismatch");
        assert_eq!(sc, dc, "content mismatch for {sr}");
    }
}

#[test]
fn test_plain_roundtrip() {
    let src = TempDir::new().unwrap();
    let tmp = TempDir::new().unwrap();
    let dst = TempDir::new().unwrap();

    write_tree(src.path());
    let archive = tmp.path().join("out.arx");

    pack(&[src.path()], &archive, None).expect("pack failed");
    assert!(archive.exists());

    extract(&archive, dst.path(), None).expect("extract failed");
    compare_trees(src.path(), dst.path());
}

#[test]
fn test_deterministic_roundtrip() {
    let src = TempDir::new().unwrap();
    let tmp = TempDir::new().unwrap();
    let dst = TempDir::new().unwrap();

    write_tree(src.path());
    let archive = tmp.path().join("det.arx");

    let opts = PackOptions {
        deterministic: true,
        min_gain: 0.05,
        ..Default::default()
    };
    pack(&[src.path()], &archive, Some(&opts)).expect("deterministic pack failed");

    extract(&archive, dst.path(), None).expect("extract failed");
    compare_trees(src.path(), dst.path());

    // A second pack of the same data should produce the same archive bytes
    let archive2 = tmp.path().join("det2.arx");
    pack(&[src.path()], &archive2, Some(&opts)).expect("second deterministic pack failed");
    let b1 = fs::read(&archive).unwrap();
    let b2 = fs::read(&archive2).unwrap();
    assert_eq!(b1, b2, "deterministic packs should be byte-identical");
}

#[test]
fn test_encrypted_roundtrip() {
    let src = TempDir::new().unwrap();
    let tmp = TempDir::new().unwrap();
    let dst = TempDir::new().unwrap();

    write_tree(src.path());
    let archive = tmp.path().join("enc.arx");
    let key = [0x42u8; 32];

    let pack_opts = PackOptions {
        aead_key: Some(key),
        ..Default::default()
    };
    pack(&[src.path()], &archive, Some(&pack_opts)).expect("encrypted pack failed");

    // Extract with correct key → success
    let ext_opts = ExtractOptions {
        aead_key: Some(key),
        ..Default::default()
    };
    extract(&archive, dst.path(), Some(&ext_opts)).expect("encrypted extract failed");
    compare_trees(src.path(), dst.path());
}

#[test]
fn test_encrypted_wrong_key_fails() {
    let src = TempDir::new().unwrap();
    let tmp = TempDir::new().unwrap();
    let dst = TempDir::new().unwrap();

    fs::write(src.path().join("secret.txt"), b"top secret").unwrap();
    let archive = tmp.path().join("enc.arx");
    let key = [0x11u8; 32];
    let wrong_key = [0x22u8; 32];

    let pack_opts = PackOptions {
        aead_key: Some(key),
        ..Default::default()
    };
    pack(&[src.path()], &archive, Some(&pack_opts)).unwrap();

    let ext_opts = ExtractOptions {
        aead_key: Some(wrong_key),
        ..Default::default()
    };
    let result = extract(&archive, dst.path(), Some(&ext_opts));
    assert!(result.is_err(), "extract with wrong key should fail");
    let err_str = result.unwrap_err().to_string();
    assert!(
        err_str.contains("AEAD") || err_str.contains("aead") || err_str.contains("authentication"),
        "error should mention AEAD: {err_str}"
    );
}

#[test]
fn test_password_roundtrip() {
    let src = TempDir::new().unwrap();
    let tmp = TempDir::new().unwrap();
    let dst = TempDir::new().unwrap();

    fs::write(src.path().join("file.txt"), b"password protected").unwrap();
    let archive = tmp.path().join("pw.arx");

    let pack_opts = PackOptions {
        password: Some("hunter2".into()),
        ..Default::default()
    };
    pack(&[src.path()], &archive, Some(&pack_opts)).expect("password pack failed");

    // Extract using the password (resolved via superblock kdf_salt)
    let ext_opts = ExtractOptions {
        password: Some("hunter2".into()),
        ..Default::default()
    };
    extract(&archive, dst.path(), Some(&ext_opts)).expect("password extract failed");
    let content = fs::read(dst.path().join("file.txt")).unwrap();
    assert_eq!(content, b"password protected");
}
