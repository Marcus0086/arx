/// CRUD overlay integration tests.
use arx_core::crud::CrudArchive;
use arx_core::read::extract::extract;
use std::fs;
use std::io::Read;
use tempfile::TempDir;

fn issue(out: &std::path::Path) {
    CrudArchive::issue_archive(
        out,
        "test",
        "tester",
        "integration test",
        None,
        [0u8; 32],
        true,
    )
    .expect("issue_archive failed");
}

#[test]
fn test_put_and_ls() {
    let tmp = TempDir::new().unwrap();
    let archive = tmp.path().join("base.arx");
    issue(&archive);

    let src = tmp.path().join("hello.txt");
    fs::write(&src, b"hello from crud").unwrap();

    let mut arc = CrudArchive::open(&archive).unwrap();
    arc.put_file(&src, "hello.txt", 0o644, 1000).unwrap();

    assert!(
        arc.index.by_path.contains_key("hello.txt"),
        "file should be in index"
    );
    let entry = &arc.index.by_path["hello.txt"];
    assert_eq!(entry.size, 15);
}

#[test]
fn test_put_and_read_back() {
    let tmp = TempDir::new().unwrap();
    let archive = tmp.path().join("base.arx");
    issue(&archive);

    let content = b"the quick brown fox jumps over the lazy dog";
    let src = tmp.path().join("fox.txt");
    fs::write(&src, content).unwrap();

    let mut arc = CrudArchive::open(&archive).unwrap();
    arc.put_file(&src, "fox.txt", 0o644, 2000).unwrap();

    let mut r = arc.open_reader("fox.txt").unwrap();
    let mut buf = Vec::new();
    r.read_to_end(&mut buf).unwrap();
    assert_eq!(buf, content);
}

#[test]
fn test_delete_removes_from_index() {
    let tmp = TempDir::new().unwrap();
    let archive = tmp.path().join("base.arx");
    issue(&archive);

    let src = tmp.path().join("file.txt");
    fs::write(&src, b"to be deleted").unwrap();

    let mut arc = CrudArchive::open(&archive).unwrap();
    arc.put_file(&src, "file.txt", 0o644, 1000).unwrap();
    assert!(arc.index.by_path.contains_key("file.txt"));

    arc.delete_path("file.txt").unwrap();
    assert!(
        !arc.index.by_path.contains_key("file.txt"),
        "file should be deleted"
    );
}

#[test]
fn test_rename() {
    let tmp = TempDir::new().unwrap();
    let archive = tmp.path().join("base.arx");
    issue(&archive);

    let src = tmp.path().join("orig.txt");
    fs::write(&src, b"renamed").unwrap();

    let mut arc = CrudArchive::open(&archive).unwrap();
    arc.put_file(&src, "orig.txt", 0o644, 1000).unwrap();
    arc.rename("orig.txt", "new.txt").unwrap();

    assert!(
        !arc.index.by_path.contains_key("orig.txt"),
        "old path should be gone"
    );
    assert!(
        arc.index.by_path.contains_key("new.txt"),
        "new path should exist"
    );
}

#[test]
fn test_sync_produces_extractable_archive() {
    let tmp = TempDir::new().unwrap();
    let archive = tmp.path().join("base.arx");
    issue(&archive);

    let content = b"synced content";
    let src = tmp.path().join("synced.txt");
    fs::write(&src, content).unwrap();

    {
        let mut arc = CrudArchive::open(&archive).unwrap();
        arc.put_file(&src, "synced.txt", 0o644, 1000).unwrap();
    }

    let synced = tmp.path().join("synced.arx");
    CrudArchive::sync_to_base(&archive, Some(&synced), true, 0.05, None, [0u8; 32], false)
        .expect("sync_to_base failed");

    let dst = TempDir::new().unwrap();
    extract(&synced, dst.path(), None).expect("extract of synced archive failed");
    let extracted = fs::read(dst.path().join("synced.txt")).unwrap();
    assert_eq!(extracted, content);
}

/// Regression test: 0-byte files must not crash open_reader or sync_to_base.
/// Iterator::all() returns true vacuously on empty slices, which previously
/// caused all_base to be true, delegating to the base archive where the file
/// doesn't exist (Bookmarks_V5.sqlite-wal style bug).
#[test]
fn test_empty_file_sync() {
    let tmp = TempDir::new().unwrap();
    let archive = tmp.path().join("base.arx");
    issue(&archive);

    let empty = tmp.path().join("empty.sqlite-wal");
    fs::write(&empty, b"").unwrap(); // 0 bytes

    {
        let mut arc = CrudArchive::open(&archive).unwrap();
        arc.put_file(&empty, "empty.sqlite-wal", 0o644, 0).unwrap();

        // open_reader on empty file must succeed and return 0 bytes
        let mut r = arc.open_reader("empty.sqlite-wal").unwrap();
        let mut buf = Vec::new();
        r.read_to_end(&mut buf).unwrap();
        assert!(buf.is_empty(), "empty file must read back as 0 bytes");
    }

    // sync_to_base must succeed even when vault contains 0-byte files
    let synced = tmp.path().join("synced.arx");
    CrudArchive::sync_to_base(&archive, Some(&synced), true, 0.05, None, [0u8; 32], false)
        .expect("sync_to_base must handle 0-byte files");

    let dst = TempDir::new().unwrap();
    arx_core::read::extract::extract(&synced, dst.path(), None)
        .expect("extract of synced archive with 0-byte file must succeed");
    let extracted = fs::read(dst.path().join("empty.sqlite-wal")).unwrap();
    assert!(extracted.is_empty());
}

#[test]
fn test_diff_shows_changes() {
    let tmp = TempDir::new().unwrap();
    let archive = tmp.path().join("base.arx");
    issue(&archive);

    let src = tmp.path().join("new_file.txt");
    fs::write(&src, b"brand new").unwrap();

    let mut arc = CrudArchive::open(&archive).unwrap();
    arc.put_file(&src, "new_file.txt", 0o644, 1000).unwrap();

    let diff = arc.diff();
    let added: Vec<_> = diff.iter().filter(|e| e.kind == "A").collect();
    assert_eq!(added.len(), 1, "should have one added file");
    assert_eq!(added[0].path, "new_file.txt");
}
