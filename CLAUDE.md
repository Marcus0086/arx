# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

**ARX** is a modern archive format written in Rust that sits between traditional archives (zip, tar.gz) and cloud storage. It provides streaming compression, built-in encryption, random access, and append-only CRUD in a single portable file.

## Build & Development Commands

```bash
# Standard Cargo
cargo build --release          # Build all binaries
cargo check                    # Fast type/syntax check
cargo test --workspace         # Run all 41 tests (unit + integration + CLI)
cargo test -p arx-core         # Library tests only
cargo fmt                      # Format code
cargo clippy                   # Lint

# Nix (preferred for reproducible builds)
nix develop                    # Enter dev shell with pinned Rust toolchain
nix build .                    # Reproducible build
nix run .                      # Run arx binary

# gRPC server
ROOT_DIR=/tmp/arx-data cargo run -p arx-grpc
```

The Nix dev shell configures sccache automatically (`RUSTC_WRAPPER`, `CARGO_HOME`, `SCCACHE_DIR`).

## Workspace Structure

Three crates:
- **`arx-core`** — library; all archive logic, zero unsafe code (`#![forbid(unsafe_code)]`)
- **`arxdev`** — CLI binary (`arxdev`, invoked as `arx`); thin presentation/application layer over arx-core
- **`arx-grpc`** — gRPC server; multi-tenant streaming service over arx-core

## arx-core Architecture

### On-disk file layout (v4, 80-byte header)
```
[Superblock 80B] → [Manifest (CBOR)] → [Chunk Table] → [Chunk Data] → [Tail Summary]
```
- **Superblock** (`container/superblock.rs`) — magic `ARXALP`, VERSION=4, offsets, flags, `kdf_salt: [u8; 32]`
- **Manifest** — CBOR-serialized file/dir/symlink metadata with optional `label`, `owner`, `notes` in `Meta`
- **Chunk Table** — 64-byte entries: codec, u_size, c_size, data_off, **blake3 hash** (v3 was 32B, no hash)
- **Tail Summary** — region-level blake3 integrity at EOF (optional but always written)

v3 archives (48-byte header, no blake3 in chunk table) can be **read** but not written. Version is detected from the superblock `version` field; callers don't need to branch.

### Encryption
Each region (manifest, chunk table, each data chunk) is sealed independently with XChaCha20-Poly1305. The **kdf_salt** is stored in the superblock (auto-generated randomly at pack time). Keys come from:
- `--encrypt-raw <32-byte-hex>` — pass raw key directly
- `--password <string>` — Argon2id KDF uses the stored `kdf_salt` (m=64MiB, t=3, p=4)

The `--key-salt` flag no longer exists at the CLI level; the salt is always read from the superblock.

### Key modules

| Module | Role |
|--------|------|
| `container/` | On-disk format: superblock, manifest, chunktab, tail, journal, delta |
| `pack/` | Archive creation — filesystem walk, FastCDC chunking, compression planning, manifest encoding |
| `read/` | Extraction (with per-chunk blake3 verification), streaming via lock-free `read_exact_at`, verification |
| `crud/` | Append-only overlay: journal (CBOR log) + delta (sidecar data). Index rebuilt from base + journal on open. |
| `chunking/fastcdc` | Content-defined chunking (min=64KiB, avg=256KiB, max=1MiB) |
| `codec/` | Pluggable compression: Store (id=0), Zstd (id=1) |
| `crypto/aead` | XChaCha20-Poly1305; `open_whole` returns `Result<Vec<u8>>` — never panics |
| `crypto/kdf` | Argon2id password → key derivation |
| `crypto/nonce` | `random_salt()` via OS CSPRNG |
| `util/varint` | LEB-128 varint encode/decode (used by journal/delta framing) |
| `util/sanitize` | `safe_join()` — path traversal prevention |
| `util/buf` | `read_exact_at()` — lock-free positional file reads via `pread` |
| `index/inmem` | In-memory BTreeMap index; `from_base()` loads base manifest, journal replays on top |
| `repo/` | `ArchiveRepo` trait + `FsArchiveRepo` impl |

### CRUD design
Write operations never mutate the base archive. An overlay composes a **journal** (CBOR operation log, `.arx.log`) + **delta** (sidecar data chunks, `.arx.delta`). Opening a `CrudArchive` loads the base manifest into `InMemIndex`, then replays the journal. `sync` compacts the overlay into a fresh base archive; the default is in-place (`.arx.tmp` + rename).

**Known behaviour**: paths stored with a leading `/` (e.g. `crud add … /foo.txt`) get the slash stripped after a `sync`, because `pack()` relativizes paths from the temp directory. Design is consistent — paths are always relative within archives.

## arxdev CLI

```
presentation/cli.rs   → clap command definitions
application/mod.rs    → dispatch
application/handlers.rs → handlers (call arx-core; resolve keys via superblock kdf_salt)
```

### Key/password resolution (all commands that accept `--key`/`--password`)
Handlers call `resolve_key(archive, key_hex, password)`:
1. If `--key <hex>` given → use as raw 32-byte key
2. If `--password <string>` given → open archive superblock, read `kdf_salt`, derive key via Argon2id
3. Neither → no encryption

Error handling: wrong key/password → `ArxError::AeadError`; not `AeadError` wrapping Io panics.

### Commands quick reference
```bash
arx pack [--encrypt-raw KEY | --password PW] [--deterministic] [--min-gain 0.05] OUT INPUTS…
arx list [--key KEY | --password PW] ARCHIVE
arx extract [--key KEY | --password PW] ARCHIVE DEST
arx verify [--key KEY | --password PW] ARCHIVE
arx issue [--encrypt-raw KEY | --password PW] [--label STR] [--owner STR] [--notes STR] OUT

arx chunk chunks ARCHIVE PATH [--key KEY | --password PW]
arx chunk cat    ARCHIVE PATH [--start N] [--len N] [--key KEY | --password PW]
arx chunk get    ARCHIVE PATH OUT [--start N] [--len N] [--key KEY | --password PW]

arx crud add  ARCHIVE SRC DST [--recursive] [--mode OCTAL] [--mtime EPOCH] [--key KEY | --password PW]
arx crud rm   ARCHIVE PATH [--recursive] [--key KEY | --password PW]
arx crud mv   ARCHIVE FROM TO [--key KEY | --password PW]
arx crud ls   ARCHIVE [--prefix STR] [--long] [--key KEY | --password PW]
arx crud diff ARCHIVE [--key KEY | --password PW]
arx crud sync ARCHIVE [--out PATH] [--seal-base] [--min-gain 0.05] [--key KEY | --password PW]
arx crud cat  ARCHIVE PATH [--key KEY | --password PW]
arx crud get  ARCHIVE PATH OUT [--key KEY | --password PW]
```

## arx-grpc Server

Multi-tenant gRPC server (port 50051 by default). Uses streaming RPCs for pack (upload) and extract (download).

```bash
# Start (dev — data stored in ~/.arx-grpc or CWD/.arx-grpc)
cargo run -p arx-grpc

# Start with explicit config
ROOT_DIR=/var/lib/arx-grpc PORT=50051 TENANTS_JSON=/etc/arx/tenants.json cargo run -p arx-grpc
```

**Auth**: `authorization: Bearer <api_key>` header in every request. Map keys → tenant UUIDs in `tenants.json`.

**Proto**: `arx-grpc/proto/arx.proto`. Generated types are pre-compiled in `src/arx_gen.rs` (no `protoc` required). Regenerate with: `PROTOC=$(which protoc) cargo build -p arx-grpc`.

## Test Suite

```bash
cargo test --workspace     # 41 tests total
cargo test -p arx-core     # 21 unit + 14 integration
```

Test locations:
- Inline `#[cfg(test)]` in: `superblock`, `chunktab`, `manifest`, `aead`, `kdf`, `varint`, `sanitize`
- `arx-core/tests/round_trip.rs` — pack→extract→diff, deterministic, encrypted, password
- `arx-core/tests/chunk_integrity.rs` — corruption detection (extract + verify)
- `arx-core/tests/crud_ops.rs` — full CRUD workflow including sync and diff
- `arxdev/tests/cli.rs` — 6 end-to-end CLI tests via `process::Command`

## Notable Constraints

- **Rust edition 2024** — use current idioms.
- **Deterministic mode** (`--deterministic`) — zeroes timestamps and kdf_salt; relevant when touching manifest or superblock serialization.
- **v3 backward compat** — read works; v3 chunk entries have `blake3 = [0u8; 32]` (no integrity check). All writes produce v4.
- **Symlinks** — `allow_symlinks: bool` in `Policy`; symlink walk and restore are implemented for Unix.
- **No protoc needed** — arx-grpc compiles from pre-generated `src/arx_gen.rs`.
