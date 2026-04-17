/// CLI integration tests — invoke the compiled `arx` binary via std::process::Command.
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use tempfile::TempDir;

fn arx_bin() -> PathBuf {
    // CARGO_BIN_EXE_arxdev is set by cargo test for integration tests
    env!("CARGO_BIN_EXE_arxdev").into()
}

fn arx(args: &[&str]) -> std::process::Output {
    Command::new(arx_bin())
        .args(args)
        .output()
        .expect("failed to run arx binary")
}

fn assert_success(out: &std::process::Output) {
    assert!(
        out.status.success(),
        "command failed.\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr),
    );
}

fn write_fixtures(root: &std::path::Path) {
    fs::create_dir_all(root.join("sub")).unwrap();
    fs::write(root.join("hello.txt"), b"hello world\n").unwrap();
    fs::write(root.join("sub/data.bin"), &[0xAAu8; 1024]).unwrap();
}

fn compare_trees(src: &std::path::Path, dst: &std::path::Path) {
    let mut src_files: Vec<_> = walkdir::WalkDir::new(src)
        .sort_by_file_name()
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .map(|e| {
            let rel = e
                .path()
                .strip_prefix(src)
                .unwrap()
                .to_string_lossy()
                .into_owned();
            let content = fs::read(e.path()).unwrap();
            (rel, content)
        })
        .collect();
    let mut dst_files: Vec<_> = walkdir::WalkDir::new(dst)
        .sort_by_file_name()
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .map(|e| {
            let rel = e
                .path()
                .strip_prefix(dst)
                .unwrap()
                .to_string_lossy()
                .into_owned();
            let content = fs::read(e.path()).unwrap();
            (rel, content)
        })
        .collect();
    src_files.sort_by(|a, b| a.0.cmp(&b.0));
    dst_files.sort_by(|a, b| a.0.cmp(&b.0));
    assert_eq!(
        src_files.len(),
        dst_files.len(),
        "file count mismatch between src and dst"
    );
    for ((sr, sc), (dr, dc)) in src_files.iter().zip(dst_files.iter()) {
        assert_eq!(sr, dr, "path mismatch");
        assert_eq!(sc, dc, "content mismatch for {sr}");
    }
}

// ── Basic pack / list / extract / verify ─────────────────────────────────────

#[test]
fn test_cli_pack_list_extract() {
    let tmp = TempDir::new().unwrap();
    let fixtures = tmp.path().join("src");
    let archive = tmp.path().join("out.arx");
    let dest = tmp.path().join("dst");

    write_fixtures(&fixtures);

    assert_success(&arx(&[
        "pack",
        archive.to_str().unwrap(),
        fixtures.to_str().unwrap(),
    ]));
    assert!(archive.exists(), "archive should exist after pack");

    let list_out = arx(&["list", archive.to_str().unwrap()]);
    assert_success(&list_out);
    let stdout = String::from_utf8_lossy(&list_out.stdout);
    assert!(stdout.contains("hello.txt"), "list should show hello.txt");

    assert_success(&arx(&[
        "extract",
        archive.to_str().unwrap(),
        dest.to_str().unwrap(),
    ]));
    compare_trees(&fixtures, &dest);
}

#[test]
fn test_cli_verify() {
    let tmp = TempDir::new().unwrap();
    let fixtures = tmp.path().join("src");
    let archive = tmp.path().join("verify.arx");
    fs::write(
        fixtures
            .join("..")
            .join("src")
            .to_path_buf()
            .join("f.txt")
            .parent()
            .unwrap()
            .join("f.txt"),
        b"data",
    )
    .unwrap_or_else(|_| {
        fs::create_dir_all(&fixtures).unwrap();
        fs::write(fixtures.join("f.txt"), b"data").unwrap();
    });
    fs::create_dir_all(&fixtures).unwrap();
    fs::write(fixtures.join("f.txt"), b"data").unwrap();

    assert_success(&arx(&[
        "pack",
        archive.to_str().unwrap(),
        fixtures.to_str().unwrap(),
    ]));
    assert_success(&arx(&["verify", archive.to_str().unwrap()]));
}

// ── Encrypted pack / extract ─────────────────────────────────────────────────

#[test]
fn test_cli_pack_extract_encrypted_raw_key() {
    let tmp = TempDir::new().unwrap();
    let fixtures = tmp.path().join("src");
    let archive = tmp.path().join("enc.arx");
    let dest = tmp.path().join("dst");
    let key = "deadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeef";

    fs::create_dir_all(&fixtures).unwrap();
    fs::write(fixtures.join("secret.txt"), b"encrypted content").unwrap();

    assert_success(&arx(&[
        "pack",
        archive.to_str().unwrap(),
        fixtures.to_str().unwrap(),
        "--encrypt-raw",
        key,
    ]));

    // Extract without key should fail
    let bad = arx(&["extract", archive.to_str().unwrap(), dest.to_str().unwrap()]);
    assert!(!bad.status.success(), "extract without key should fail");

    // Extract with correct key
    assert_success(&arx(&[
        "extract",
        archive.to_str().unwrap(),
        dest.to_str().unwrap(),
        "--key",
        key,
    ]));
    assert_eq!(
        fs::read(dest.join("secret.txt")).unwrap(),
        b"encrypted content"
    );
}

#[test]
fn test_cli_pack_extract_password() {
    let tmp = TempDir::new().unwrap();
    let fixtures = tmp.path().join("src");
    let archive = tmp.path().join("pw.arx");
    let dest = tmp.path().join("dst");

    fs::create_dir_all(&fixtures).unwrap();
    fs::write(fixtures.join("pw.txt"), b"password protected").unwrap();

    assert_success(&arx(&[
        "pack",
        archive.to_str().unwrap(),
        fixtures.to_str().unwrap(),
        "--password",
        "hunter2",
    ]));

    assert_success(&arx(&[
        "extract",
        archive.to_str().unwrap(),
        dest.to_str().unwrap(),
        "--password",
        "hunter2",
    ]));
    assert_eq!(
        fs::read(dest.join("pw.txt")).unwrap(),
        b"password protected"
    );
}

// ── CRUD commands ─────────────────────────────────────────────────────────────

#[test]
fn test_cli_crud_add_ls_diff() {
    let tmp = TempDir::new().unwrap();
    let archive = tmp.path().join("crud.arx");
    let src = tmp.path().join("new_file.txt");

    // Issue empty archive
    assert_success(&arx(&[
        "issue",
        archive.to_str().unwrap(),
        "--label",
        "test",
    ]));

    fs::write(&src, b"crud add content").unwrap();

    // Add file
    assert_success(&arx(&[
        "crud",
        "add",
        archive.to_str().unwrap(),
        src.to_str().unwrap(),
        "new_file.txt",
    ]));

    // Ls should show it
    let ls_out = arx(&["crud", "ls", archive.to_str().unwrap()]);
    assert_success(&ls_out);
    let stdout = String::from_utf8_lossy(&ls_out.stdout);
    assert!(
        stdout.contains("new_file.txt"),
        "ls should show new_file.txt"
    );

    // Diff should show it as Added
    let diff_out = arx(&["crud", "diff", archive.to_str().unwrap()]);
    assert_success(&diff_out);
    let stdout = String::from_utf8_lossy(&diff_out.stdout);
    assert!(stdout.contains('A'), "diff should show A for added file");
}

#[test]
fn test_cli_crud_sync() {
    let tmp = TempDir::new().unwrap();
    let archive = tmp.path().join("base.arx");
    let synced = tmp.path().join("synced.arx");
    let src = tmp.path().join("f.txt");
    let dest = tmp.path().join("dst");

    assert_success(&arx(&[
        "issue",
        archive.to_str().unwrap(),
        "--label",
        "sync-test",
    ]));

    fs::write(&src, b"sync me").unwrap();
    assert_success(&arx(&[
        "crud",
        "add",
        archive.to_str().unwrap(),
        src.to_str().unwrap(),
        "f.txt",
    ]));

    assert_success(&arx(&[
        "crud",
        "sync",
        archive.to_str().unwrap(),
        "--out",
        synced.to_str().unwrap(),
    ]));

    assert!(synced.exists(), "synced archive should exist");

    assert_success(&arx(&[
        "extract",
        synced.to_str().unwrap(),
        dest.to_str().unwrap(),
    ]));
    assert_eq!(fs::read(dest.join("f.txt")).unwrap(), b"sync me");
}
