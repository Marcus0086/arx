# ARX

**ARX is a modern archive format that acts like a compressed, encrypted object store in a single file.**  
It combines the simplicity of `.zip` with the power of S3: content-defined chunking, Zstd compression, per-region AEAD encryption, and append-only semantics for safe CRUD.

---

## ✨ Why ARX?

Traditional archives (`zip`, `tar.gz`) are showing their age:
- ❌ No random access once compressed/encrypted  
- ❌ Deletes and updates require rewriting the whole file  
- ❌ No deduplication across files or archives  
- ❌ Bolt-on encryption, not streaming-friendly  

Cloud stores (S3, MinIO) solve these but at a cost:
- Heavy operational overhead  
- External dependencies  
- Not portable  

👉 **ARX is the missing middle.**  
It gives you **streaming compression, built-in encryption, random access, and append-only updates** — all in one portable `.arx` file.

---

## 🔑 Key Features

- **Streaming by default** – Pack/extract TB-scale data without loading whole files into RAM.  
- **Content-defined chunking (FastCDC)** – Natural deduplication of similar files.  
- **Zstd compression (with STORE fallback)** – High ratio when possible, zero penalty when not.  
- **Per-region AEAD encryption** – Random access into encrypted archives with XChaCha20-Poly1305.  
- **Append-only CRUD** – Add, update, delete (via tombstones) without rewriting everything.  
- **Deterministic mode** – Reproducible builds for supply chain & CI/CD.  
- **Path sanitization & policies** – Security-first: reject unsafe paths, enforce size/ratio caps.  
- **Extensible codec registry** – Plug in new compressors/filters without changing the format.  

---

## 📦 Getting Started

### Build from source (alpha)
```bash
git clone https://github.com/your-org/arx.git
cd arx
cargo build --release
```

The binary will be available at:
```
target/release/arx
```

---

### Pack files into an archive
```bash
./target/release/arx pack out.arx ./my_project
```

### List archive contents
```bash
./target/release/arx list out.arx
```

### Extract archive
```bash
./target/release/arx extract out.arx ./dest
```

### Delete a file (soft delete / tombstone)
```bash
./target/release/arx rm out.arx secret.txt
```

### Compact (purge deleted data)
```bash
./target/release/arx compact out.arx
```

---

## 🏗 Architecture

```
File Reader ─▶ Chunker(FastCDC) ─▶ Compressor(Zstd/Store) ─▶ AEAD Seal (optional) ─▶ Sink
                      │
                      └──▶ Chunk Table (offsets, codec, sizes)
```

**On-disk layout:**
```
Superblock → Manifest (CBOR) → Chunk Table → Chunk Data Segments → Tail Summary
```

- **Append-only:** new manifests/chunks are appended, preserving old generations.  
- **Encryption:** each region (manifest, table, data) sealed independently.  
- **Random access:** locate chunks via table, decrypt only the needed region.  

---

## 🛠 API

Rust library API:

```rust
fn pack(archive: &mut ArchiveWriter, inputs: impl Iterator<Item=InputSpec>) -> Result<()>;
fn list(archive: &ArchiveReader) -> Result<Vec<Entry>>;
fn extract(archive: &ArchiveReader, spec: ExtractSpec) -> Result<()>;
fn rm(archive: &mut ArchiveWriter, paths: &[Path], mode: DeleteMode) -> Result<()>;
fn compact(src: &ArchiveReader, dst: &mut ArchiveWriter) -> Result<()>;
```

---

## 🔒 Security

- **AEAD everywhere:** XChaCha20-Poly1305 seals each region independently.  
- **Key derivation:** tenant root key → vault key → archive region keys (HKDF).  
- **Tamper detection:** decryption fails on modification.  
- **Sanitizer:** rejects absolute paths, `..`, and unsafe symlinks.  

---

## 📊 Roadmap

- ✅ v0: Store/Zstd codecs, AEAD, append-only CRUD  
- 🔜 Snapshots & time-travel  
- 🔜 Cross-archive dedup (persistent chunk DB)  
- 🔜 Cloud range I/O (HTTP/S3)  
- 🔜 WASM/C-ABI plugin system  

---

## 🤔 When to Use ARX?

- **Backups** – Append-only, encrypted, deduplicated archives for projects or servers.  
- **CI/CD Artifacts** – Reproducible, deterministic packaging with policy caps.  
- **Portable Datasets** – Ship compressed, encrypted datasets with random-access reads.  
- **Self-hosted Vaults** – S3-like storage in a single portable file.  

---

## 📜 License

MIT License - see [LICENSE](LICENSE)

**ARX — the archive that thinks like a storage system.**
