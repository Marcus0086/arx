/// Synthetic perturbation test suite.
///
/// Each test:
///   1. Creates a valid archive (or CRUD overlay).
///   2. Injects a specific fault at the binary level.
///   3. Attempts the relevant operation.
///   4. Asserts `Err(...)` is returned — never a panic.
///
/// Run: `cargo test -p arx-core perturb`
use arx_core::container::superblock::Superblock;
use arx_core::crud::CrudArchive;
use arx_core::read::extract::extract;
use arx_core::read::opened::Opened;
use arx_core::{PackOptions, pack};
use std::fs::{self, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::Path;
use tempfile::TempDir;

// ── Helpers ──────────────────────────────────────────────────────────────────

fn make_archive(dir: &Path, content: &[u8]) -> std::path::PathBuf {
    let src = dir.join("src_dir");
    fs::create_dir_all(&src).unwrap();
    fs::write(src.join("data.bin"), content).unwrap();
    let archive = dir.join("base.arx");
    pack(
        &[src.as_path()],
        &archive,
        Some(&PackOptions {
            deterministic: true,
            ..Default::default()
        }),
    )
    .unwrap();
    archive
}

fn make_crud_archive(dir: &Path, content: &[u8]) -> std::path::PathBuf {
    let archive = dir.join("base.arx");
    CrudArchive::issue_archive(&archive, "test", "tester", "", None, [0u8; 32], true).unwrap();
    {
        let src = dir.join("file.bin");
        fs::write(&src, content).unwrap();
        let mut arc = CrudArchive::open(&archive).unwrap();
        arc.put_file(&src, "file.bin", 0o644, 0).unwrap();
    }
    archive
}

fn read_superblock(archive: &Path) -> Superblock {
    let mut f = fs::File::open(archive).unwrap();
    Superblock::read_from(&mut f).unwrap()
}

fn patch_bytes(path: &Path, offset: u64, bytes: &[u8]) {
    let mut f = OpenOptions::new().write(true).open(path).unwrap();
    f.seek(SeekFrom::Start(offset)).unwrap();
    f.write_all(bytes).unwrap();
}

fn read_bytes_at(path: &Path, offset: u64, n: usize) -> Vec<u8> {
    let mut f = fs::File::open(path).unwrap();
    f.seek(SeekFrom::Start(offset)).unwrap();
    let mut buf = vec![0u8; n];
    f.read_exact(&mut buf).unwrap();
    buf
}

fn flip_byte(path: &Path, offset: u64) {
    let b = read_bytes_at(path, offset, 1);
    patch_bytes(path, offset, &[b[0] ^ 0xFF]);
}

fn truncate_file(path: &Path, new_len: u64) {
    let f = OpenOptions::new().write(true).open(path).unwrap();
    f.set_len(new_len).unwrap();
}

// ── ARCHIVE FORMAT PERTURBATIONS ─────────────────────────────────────────────

/// Flip the archive magic — Opened::open must return Err, not panic.
#[test]
fn perturb_bad_magic() {
    let tmp = TempDir::new().unwrap();
    let archive = make_archive(tmp.path(), b"hello perturb");
    patch_bytes(&archive, 0, b"BADMAG");
    let result = Opened::open(&archive, None, [0u8; 32]);
    assert!(result.is_err(), "bad magic should return Err");
    let msg = result.err().unwrap().to_string();
    assert!(
        msg.contains("magic") || msg.contains("ARXALP") || msg.contains("bad"),
        "{msg}"
    );
}

/// Set version to an unknown value (99) — Opened::open must return Err.
#[test]
fn perturb_unknown_version() {
    let tmp = TempDir::new().unwrap();
    let archive = make_archive(tmp.path(), b"version perturb");
    // Version field is at offset 6, u16 little-endian
    patch_bytes(&archive, 6, &99u16.to_le_bytes());
    let result = Opened::open(&archive, None, [0u8; 32]);
    assert!(result.is_err(), "unknown version should return Err");
    let msg = result.err().unwrap().to_string();
    assert!(msg.contains("version") || msg.contains("99"), "{msg}");
}

/// Set manifest_len to 512 MiB in the superblock — Opened::open must return
/// Err(manifest_len exceeds maximum) without allocating 512 MiB.
#[test]
fn perturb_huge_manifest_len() {
    let tmp = TempDir::new().unwrap();
    let archive = make_archive(tmp.path(), b"oom test");
    // manifest_len at offset 8, u64 little-endian
    let huge: u64 = 512 * 1024 * 1024; // 512 MiB
    patch_bytes(&archive, 8, &huge.to_le_bytes());
    let result = Opened::open(&archive, None, [0u8; 32]);
    assert!(
        result.is_err(),
        "huge manifest_len should return Err, not OOM"
    );
    let msg = result.err().unwrap().to_string();
    assert!(msg.contains("manifest") || msg.contains("maximum"), "{msg}");
}

/// Set manifest_len to u64::MAX — must return Err, not allocate.
#[test]
fn perturb_manifest_len_max() {
    let tmp = TempDir::new().unwrap();
    let archive = make_archive(tmp.path(), b"max manifest");
    patch_bytes(&archive, 8, &u64::MAX.to_le_bytes());
    let result = Opened::open(&archive, None, [0u8; 32]);
    assert!(result.is_err(), "manifest_len=MAX should return Err");
}

/// Corrupt a chunk table entry's chunk_id to out-of-bounds value, then
/// attempt to open a file — must return Err, not panic on table[id].
#[test]
fn perturb_invalid_chunk_id_in_manifest() {
    let tmp = TempDir::new().unwrap();
    let archive = make_archive(tmp.path(), &vec![0xABu8; 4096]);
    let _sb = read_superblock(&archive);
    // Each chunk table entry is 64 bytes in v4.
    // At chunk_table_off, plaintext entry 0 starts.
    // In our test archive (unencrypted), the table_bytes are written directly
    // after the manifest at chunk_table_off.
    // Entry layout: [codec:1][padding:7][u_size:8][c_size:8][data_off:8][blake3:32] = 64B
    // Set the chunk ID in the manifest's chunk_refs to an out-of-bounds value.
    // Actually, the manifest stores chunk_ids in CBOR. Easier to corrupt
    // the chunk_count field in the superblock to be smaller than actual,
    // then read — this makes valid chunk_ids exceed the (now smaller) table.
    // Set chunk_count to 0 but keep data: any chunk reference in manifest
    // would be >= table.len() == 0.
    patch_bytes(&archive, 24, &0u64.to_le_bytes()); // chunk_count = 0
    // Now from_base() will try to index table[id] where table is empty → bounds error
    let result = Opened::open(&archive, None, [0u8; 32]);
    // Opened may succeed (0 entries in table is valid for empty archive),
    // but InMemIndex::from_base should fail when manifest has chunk refs > table.
    // If Opened succeeds, list the entries to trigger the check.
    if let Ok(opened) = result {
        let result = arx_core::index::inmem::InMemIndex::from_base(&opened);
        assert!(
            result.is_err(),
            "chunk id out of bounds should return Err, not panic"
        );
    }
    // Either Opened or from_base returning Err is acceptable — no panic is the key invariant.
}

/// Flip a byte in a chunk's data region — extract must return Err (blake3 mismatch).
#[test]
fn perturb_flip_chunk_data_byte() {
    let tmp = TempDir::new().unwrap();
    let archive = make_archive(tmp.path(), &vec![0x55u8; 8192]);
    let sb = read_superblock(&archive);
    // Flip a byte in the first chunk's data
    flip_byte(&archive, sb.data_off + 16);
    let dst = TempDir::new().unwrap();
    let result = extract(&archive, dst.path(), None);
    assert!(result.is_err(), "corrupt chunk data should fail extract");
}

/// Truncate the archive in the middle of the data region.
#[test]
fn perturb_truncate_data_region() {
    let tmp = TempDir::new().unwrap();
    let archive = make_archive(tmp.path(), &vec![0xCCu8; 16384]);
    let sb = read_superblock(&archive);
    // Truncate to just after chunk_table_off (cuts off most of the data region)
    truncate_file(&archive, sb.data_off + 4);
    let dst = TempDir::new().unwrap();
    let result = extract(&archive, dst.path(), None);
    assert!(result.is_err(), "truncated data region should fail extract");
}

/// Set chunk_table_off past end-of-file — Opened::open must return Err.
#[test]
fn perturb_chunk_table_off_beyond_eof() {
    let tmp = TempDir::new().unwrap();
    let archive = make_archive(tmp.path(), b"table off test");
    let file_len = fs::metadata(&archive).unwrap().len();
    // chunk_table_off is at offset 16
    patch_bytes(&archive, 16, &(file_len + 1_000_000).to_le_bytes());
    let result = Opened::open(&archive, None, [0u8; 32]);
    assert!(
        result.is_err(),
        "chunk_table_off past EOF should return Err"
    );
}

// ── CRUD JOURNAL PERTURBATIONS ────────────────────────────────────────────────

/// Corrupt the journal magic — on next open, the journal should be backed up
/// to .arx.log.corrupt and a fresh journal initialized (no panic, no data loss
/// of the base archive).
#[test]
fn perturb_corrupt_journal_magic() {
    let tmp = TempDir::new().unwrap();
    let archive = make_crud_archive(tmp.path(), b"journal magic test");

    let log_path = tmp.path().join("base.arx.log");
    // Overwrite the journal magic (first 8 bytes)
    patch_bytes(&log_path, 0, b"GARBAGE\0");

    // Opening the archive must not panic; it should recover
    let result = CrudArchive::open(&archive);
    assert!(
        result.is_ok(),
        "corrupt journal magic should be handled gracefully, got: {:?}",
        result.err().map(|e| e.to_string())
    );

    // The backup file should exist
    let backup = tmp.path().join("base.arx.log.corrupt");
    assert!(
        backup.exists(),
        "corrupted journal should be backed up to .log.corrupt"
    );
}

/// Truncate the journal in the middle of a record (simulates crash mid-write).
/// CrudArchive::open must succeed and replay whatever records were complete.
#[test]
fn perturb_truncated_journal_record() {
    let tmp = TempDir::new().unwrap();
    let archive = make_crud_archive(tmp.path(), b"truncated journal test");

    let log_path = tmp.path().join("base.arx.log");
    let log_len = fs::metadata(&log_path).unwrap().len();
    // Truncate to header size + half the first record (rough approximation)
    // Journal header = 8 (magic) + 1 (version) + 1 (flags) + 32 (salt) = 42 bytes
    let header_size = 42u64;
    if log_len > header_size + 4 {
        let partial = header_size + (log_len - header_size) / 2;
        truncate_file(&log_path, partial);
    }

    // Must not panic; partial records are skipped gracefully
    let result = CrudArchive::open(&archive);
    assert!(
        result.is_ok(),
        "truncated journal should be opened gracefully"
    );
}

/// Truncate the delta file so a chunk's data is missing.
/// open_reader on the affected file should return Err, not panic.
#[test]
fn perturb_truncated_delta() {
    let tmp = TempDir::new().unwrap();
    let archive = make_crud_archive(tmp.path(), &vec![0xDDu8; 512]);

    let delta_path = tmp.path().join("base.arx.delta");
    let delta_len = fs::metadata(&delta_path).unwrap().len();
    // Cut delta in half
    if delta_len > 4 {
        truncate_file(&delta_path, delta_len / 2);
    }

    let arc = CrudArchive::open(&archive).unwrap();
    let result = arc.open_reader("file.bin");
    if let Ok(mut reader) = result {
        let mut buf = Vec::new();
        let io_result = reader.read_to_end(&mut buf);
        // Either open_reader or the subsequent read must return Err
        assert!(
            io_result.is_err(),
            "reading from truncated delta should fail"
        );
    }
    // If open_reader returned Err, that's also acceptable — key invariant: no panic
}

/// Write a partial varint (continuation byte without a final byte) at the end
/// of the journal, then open — replay must stop gracefully without panicking.
#[test]
fn perturb_partial_varint_at_journal_eof() {
    let tmp = TempDir::new().unwrap();
    let archive = make_crud_archive(tmp.path(), b"varint test");

    let log_path = tmp.path().join("base.arx.log");
    // Append an incomplete varint: 0x80 indicates "more bytes follow" but we stop there
    let mut f = OpenOptions::new().append(true).open(&log_path).unwrap();
    f.write_all(&[0x80u8]).unwrap(); // partial continuation byte
    drop(f);

    let result = CrudArchive::open(&archive);
    assert!(
        result.is_ok(),
        "partial varint at journal EOF should be handled gracefully"
    );
}

// ── RANGE READER PERTURBATIONS ────────────────────────────────────────────────

/// Request a byte range starting past the end of a file — must return Err.
#[test]
fn perturb_range_reader_start_past_eof() {
    let tmp = TempDir::new().unwrap();
    let archive = make_archive(tmp.path(), b"short file");
    let opened = Opened::open(&archive, None, [0u8; 32]).unwrap();

    // The file is "src_dir/data.bin" with 10 bytes.
    // Start at offset 1000 (way past EOF).
    let result = opened.open_range("src_dir/data.bin", 1000, 100);
    assert!(result.is_err(), "start past EOF should return Err");
}

// ── SYNC + RENAME PERTURBATIONS ───────────────────────────────────────────────

/// rename() with a non-existent source should return Err.
#[test]
fn perturb_rename_nonexistent_source() {
    let tmp = TempDir::new().unwrap();
    let archive = make_crud_archive(tmp.path(), b"rename test");
    let mut arc = CrudArchive::open(&archive).unwrap();
    let result = arc.rename("does_not_exist.txt", "target.txt");
    assert!(
        result.is_err(),
        "rename with missing source should return Err"
    );
}

/// rename() with an existing target should return Err.
#[test]
fn perturb_rename_existing_target() {
    let tmp = TempDir::new().unwrap();
    let archive = tmp.path().join("base.arx");
    CrudArchive::issue_archive(&archive, "t", "t", "", None, [0u8; 32], true).unwrap();

    // Add two files
    let a = tmp.path().join("a.txt");
    let b = tmp.path().join("b.txt");
    fs::write(&a, b"aaa").unwrap();
    fs::write(&b, b"bbb").unwrap();
    let mut arc = CrudArchive::open(&archive).unwrap();
    arc.put_file(&a, "a.txt", 0o644, 0).unwrap();
    arc.put_file(&b, "b.txt", 0o644, 0).unwrap();

    // Rename a.txt → b.txt (b.txt already exists)
    let result = arc.rename("a.txt", "b.txt");
    assert!(
        result.is_err(),
        "rename to existing target should return Err"
    );
}

/// After sync_to_base, the sidecar files (.arx.log, .arx.delta) must be gone.
#[test]
fn perturb_sync_cleans_sidecars() {
    let tmp = TempDir::new().unwrap();
    let archive = make_crud_archive(tmp.path(), b"sync cleanup test");

    let log_path = tmp.path().join("base.arx.log");
    let delta_path = tmp.path().join("base.arx.delta");
    assert!(log_path.exists(), "log should exist before sync");
    assert!(delta_path.exists(), "delta should exist before sync");

    CrudArchive::sync_to_base(&archive, None, true, 0.05, None, [0u8; 32], false).unwrap();

    assert!(!log_path.exists(), "log should be cleaned up after sync");
    assert!(
        !delta_path.exists(),
        "delta should be cleaned up after sync"
    );
}

// ── SUMMARY ──────────────────────────────────────────────────────────────────
// Run all: cargo test -p arx-core perturb
// Run one: cargo test -p arx-core perturb::perturb_bad_magic
