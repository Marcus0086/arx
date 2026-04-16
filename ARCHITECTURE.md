# ARX Architecture

This document is a complete architectural reference for the ARX codebase, covering both High-Level Design (HLD) and Low-Level Design (LLD) with Mermaid diagrams.

---

## Overview

ARX is a modern, portable archive format that sits between traditional archives (zip, tar.gz) and cloud object storage. It stores files as content-defined chunks, compressed with Zstandard, and optionally encrypted per-region with XChaCha20-Poly1305 — all inside a single binary file.

Three design principles run through the entire codebase:

1. **Immutable base, append-only overlay.** The `.arx` archive is never rewritten after creation. Mutations are accumulated in sidecar files (`.arx.log` journal + `.arx.delta` data store) and optionally compacted back to a fresh base via `sync`.
2. **Region-scoped encryption.** The manifest, chunk table, and each data chunk are each sealed independently with a deterministically derived nonce. There is no master encrypted envelope; regions can be decrypted in isolation.
3. **Content-defined chunking for deduplication.** Files are split at content-defined boundaries (FastCDC / Gear rolling hash) rather than fixed block sizes, so identical sub-file regions deduplicate across files regardless of byte offsets.

---

## High-Level Design

### HLD 1 — System Context

```mermaid
graph TB
    User["User / CI pipeline"]

    subgraph "arxdev (binary)"
        CLI["Clap CLI<br/>presentation/cli.rs"]
        APP["Command Router<br/>application/mod.rs"]
        HDL["Handlers<br/>application/handlers.rs"]
    end

    subgraph "arx-core (library)"
        PACK["pack()"]
        LIST["list()"]
        EXT["extract() / verify()"]
        CRUD["CrudArchive"]
        REPO["ArchiveRepo trait"]
    end

    subgraph "External Crates"
        BLAKE3["blake3\n(hashing)"]
        CHACHA["chacha20poly1305\n(AEAD)"]
        ZSTD["zstd\n(compression)"]
        CBOR["ciborium\n(CBOR serialization)"]
        RAYON["rayon\n(parallelism)"]
        WALK["walkdir\n(directory walk)"]
    end

    subgraph "Filesystem"
        ARX[".arx archive"]
        LOG[".arx.log journal"]
        DELTA[".arx.delta sidecar"]
    end

    User --> CLI
    CLI --> APP
    APP --> HDL
    HDL --> PACK
    HDL --> LIST
    HDL --> EXT
    HDL --> CRUD
    HDL --> REPO

    PACK --> BLAKE3
    PACK --> CHACHA
    PACK --> ZSTD
    PACK --> CBOR
    PACK --> RAYON
    PACK --> WALK

    PACK --> ARX
    LIST --> ARX
    EXT --> ARX
    CRUD --> ARX
    CRUD --> LOG
    CRUD --> DELTA
```

---

### HLD 2 — Workspace & Crate Layers

```mermaid
graph TB
    subgraph "arxdev crate (binary)"
        P["presentation/cli.rs\nClap command & arg definitions"]
        A["application/mod.rs\nRun function, dispatch"]
        H["application/handlers.rs\nOne handler per command"]
    end

    subgraph "arx-core crate (library)"
        PUB["Public API\npack · list · extract · verify\nCrudArchive · ArchiveRepo"]
        CONT["container/\nsuperblock · manifest\nchunktab · tail · journal · delta"]
        PACK2["pack/\nwriter · walker"]
        READ["read/\nextract · opened · stream"]
        CRUD2["crud/mod.rs"]
        CODEC["codec/\nStore · Zstd"]
        CHUNK["chunking/fastcdc"]
        CRYPTO["crypto/\naead · kdf · nonce"]
        IDX["index/inmem"]
        REPO2["repo/\nrepo_fs · repo_factory"]
        DOMAIN["domain.rs\nFileRow · ChunkRow"]
        POLICY["policy.rs"]
        STATS["stats.rs"]
        ERR["error.rs\nArxError"]
    end

    P --> A --> H
    H --> PUB
    PUB --> PACK2
    PUB --> READ
    PUB --> CRUD2
    PUB --> REPO2
    PACK2 --> CONT
    PACK2 --> CODEC
    PACK2 --> CHUNK
    PACK2 --> CRYPTO
    READ --> CONT
    READ --> CODEC
    READ --> CRYPTO
    CRUD2 --> CONT
    CRUD2 --> IDX
    CRUD2 --> PACK2
    REPO2 --> READ
    REPO2 --> DOMAIN
    IDX --> POLICY
    IDX --> STATS
```

---

### HLD 3 — On-Disk Archive Binary Layout

```mermaid
block-beta
  columns 5
  SB["Superblock\n48 bytes\noffset 0"]:1
  MAN["Manifest\nCBOR (variable)\noffset 48"]:1
  CT["Chunk Table\n32 B × chunk_count\noffset = chunk_table_off"]:1
  DATA["Chunk Data\nvariable\noffset = data_off"]:1
  TAIL["Tail Summary\n120 bytes\nEOF − 120 (optional)"]:1
```

**Superblock fields (48 bytes, all little-endian):**

| Bytes | Field | Value / Notes |
|-------|-------|---------------|
| 0–5 | magic | `ARXALP` (alpha marker) |
| 6–7 | version | `u16` = 3 |
| 8–15 | manifest_len | byte length of manifest region |
| 16–23 | chunk_table_off | file offset of chunk table |
| 24–31 | chunk_count | number of chunks |
| 32–39 | data_off | file offset where chunk data starts |
| 40–47 | flags | bit 0 = `FLAG_ENCRYPTED` (0x1) |

**Chunk Table entry (32 bytes per entry):**

| Bytes | Field | Notes |
|-------|-------|-------|
| 0 | codec | 0 = Store, 1 = Zstd |
| 1–7 | padding | reserved |
| 8–15 | u_size | uncompressed size |
| 16–23 | c_size | compressed size (includes 16-byte AEAD tag if encrypted) |
| 24–31 | data_off | absolute file offset to this chunk's bytes |

**Tail Summary (120 bytes):**

| Bytes | Field | Notes |
|-------|-------|-------|
| 0–7 | magic | `ARXTAIL\0` |
| 8–39 | manifest_blake3 | BLAKE3 hash of plaintext manifest |
| 40–71 | chunktab_blake3 | BLAKE3 hash of plaintext chunk table |
| 72–103 | data_blake3 | BLAKE3 hash of all plaintext chunk data |
| 104–111 | total_u | total uncompressed bytes |
| 112–119 | total_c | total compressed bytes |

---

### HLD 4 — Module Dependency Map

```mermaid
graph LR
    PW["pack/writer"]
    RE["read/extract"]
    RO["read/opened"]
    RS["read/stream"]
    LS["list"]
    CM["crud/mod"]
    RF["repo_fs"]

    SB["container/superblock"]
    MF["container/manifest"]
    TC["container/chunktab"]
    TL["container/tail"]
    JN["container/journal"]
    DT["container/delta"]

    AE["crypto/aead"]
    CD["codec/"]
    FK["chunking/fastcdc"]
    IX["index/inmem"]
    DM["domain"]

    PW --> SB
    PW --> MF
    PW --> TC
    PW --> TL
    PW --> AE
    PW --> CD
    PW --> FK

    RE --> SB
    RE --> MF
    RE --> TC
    RE --> TL
    RE --> AE
    RE --> CD

    RO --> SB
    RO --> MF
    RO --> TC
    RO --> AE

    RS --> RO
    RS --> AE
    RS --> CD

    LS --> SB
    LS --> MF
    LS --> TC
    LS --> AE

    CM --> JN
    CM --> DT
    CM --> IX
    CM --> PW

    RF --> RO
    RF --> DM
```

---

### HLD 5 — Pack Data Flow (Create Archive)

```mermaid
flowchart TD
    A["Input paths\n(files + dirs)"] --> B["WalkDir\nCollect FileEntry + DirEntry\n(sorted, deterministic optional)"]
    B --> C["Per-file: StreamingChunker\nFastCDC split\nmin=64KiB avg=256KiB max=1MiB"]
    C --> D["Per-chunk: BLAKE3 hash\nTrial compress with Zstd"]
    D --> E{"savings ≥ min_gain\n(default 5%)?"}
    E -->|Yes| F["codec = Zstd\nstore compressed bytes"]
    E -->|No| G["codec = Store\nstore raw bytes"]
    F & G --> H["Dedup by BLAKE3 hash\nFirst occurrence → new ChunkEntry\nRepeat → ChunkRef to existing"]
    H --> I["Build Manifest\nFileEntry[] + DirEntry[] + Meta\nCBOR encode"]
    I --> J["Compute layout offsets\nchunk_table_off = 48 + manifest_len\ndata_off = chunk_table_off + table_len"]
    J --> K{"Encryption enabled?"}
    K -->|Yes| L["seal manifest (Region::Manifest)\nseal chunk table (Region::ChunkTable)\nper-chunk: seal (Region::ChunkData + id)"]
    K -->|No| M["Write plaintext"]
    L & M --> N["Write Superblock stub\nWrite Manifest\nWrite Chunk Table\nWrite Chunk Data\nRewrite real Superblock"]
    N --> O["Hash plaintext regions\nWrite Tail Summary at EOF"]
```

---

### HLD 6 — Extract Data Flow

```mermaid
flowchart TD
    A["archive.arx"] --> B["Read Superblock (48B)\nValidate magic ARXALP\nRead offsets + flags"]
    B --> C{"FLAG_ENCRYPTED set?"}
    C -->|Yes| D["Read manifest bytes\nopen_whole(key, nonce_manifest)\nDecode CBOR"]
    C -->|No| E["Read manifest bytes\nDecode CBOR"]
    D & E --> F["Read Chunk Table\nDecrypt if needed\nBuild Vec<ChunkEntry>"]
    F --> G["Detect Tail (optional)\nRead 120B at EOF − 120\nCheck ARXTAIL\\0 magic"]
    G --> H["Create all dirs\n(from manifest.dirs)"]
    H --> I["For each FileEntry in manifest"]
    I --> J["For each ChunkRef in file"]
    J --> K["Seek to ChunkEntry.data_off\nRead c_size bytes"]
    K --> L{"Encrypted?"}
    L -->|Yes| M["open_whole(key,\nnonce_chunk[id])"]
    L -->|No| N["Raw bytes"]
    M & N --> O["Decompress via codec\n(Zstd decoder or Store copy)"]
    O --> P["Write to output file\naccumulate size"]
    P --> Q{"More chunks?"}
    Q -->|Yes| J
    Q -->|No| R["Verify written size\n== FileEntry.u_size"]
    R --> S["Next file"]
```

---

### HLD 7 — CRUD Overlay Architecture

```mermaid
graph TB
    subgraph "Immutable Base"
        BASE["archive.arx\nFully packed, never modified\nManifest + ChunkTable + Data + Tail"]
    end

    subgraph "Mutable Sidecars"
        LOG["archive.arx.log\nJournal (CBOR records)\nPut · Delete · Rename · SetPolicy"]
        DELTA["archive.arx.delta\nData frames for new files\nAppend-only, varint-framed"]
    end

    subgraph "Runtime"
        CRUD["CrudArchive\nHolds all three paths"]
        IDX["InMemIndex\nby_path: BTreeMap\nRebuilt by replaying journal"]
    end

    subgraph "Operations"
        PUT["put_file()\n→ append delta frame\n→ append Put record to journal\n→ update index"]
        DEL["delete_path()\n→ append Delete record\n→ mark entry deleted in index"]
        REN["rename()\n→ append Rename record\n→ update path key in index"]
        SYNC["sync_to_base()\n→ extract all live entries\n→ call pack() to new .arx\n→ sidecars discarded"]
    end

    BASE --> CRUD
    LOG --> CRUD
    DELTA --> CRUD
    CRUD --> IDX
    IDX --> PUT
    IDX --> DEL
    IDX --> REN
    IDX --> SYNC
    SYNC --> BASE
```

---

### HLD 8 — CLI Command Map

```mermaid
graph TD
    ARX["arx"] --> PACK["pack\nCreate .arx from inputs"]
    ARX --> LIST["list\nShow archive contents"]
    ARX --> EXT["extract\nUnpack to destination dir"]
    ARX --> VRY["verify\nCheck Tail Summary integrity"]
    ARX --> ISS["issue\nCreate empty archive + metadata"]
    ARX --> CHK["chunk"]
    ARX --> CRD["crud"]

    CHK --> CK1["chunks\nPrint chunk map for a file"]
    CHK --> CK2["cat\nStream file bytes to stdout\n(supports --start / --len)"]
    CHK --> CK3["get\nDownload file bytes to disk"]

    CRD --> CR1["add\nOverlay: add file or dir"]
    CRD --> CR2["rm\nOverlay: delete path / tree"]
    CRD --> CR3["mv\nOverlay: rename path"]
    CRD --> CR4["ls\nList merged state\n(--prefix, --long)"]
    CRD --> CR5["sync\nCompact overlay → new base\n(--seal-base re-encrypts)"]
    CRD --> CR6["cat\nStream overlay file to stdout"]
    CRD --> CR7["get\nDownload overlay file to disk"]
```

---

## Low-Level Design

### LLD 1 — Superblock

`arx-core/src/container/superblock.rs`

```mermaid
classDiagram
    class Superblock {
        +version: u16
        +manifest_len: u64
        +chunk_table_off: u64
        +chunk_count: u64
        +data_off: u64
        +flags: u64
        +write_to(w: impl Write) Result~()~
        +read_from(r: impl Read) Result~Superblock~
    }
    note for Superblock "MAGIC = b\"ARXALP\" (6 bytes)\nVERSION = 3\nHEADER_LEN = 48\nFLAG_ENCRYPTED = 1 << 0"
```

**On-disk bytes (little-endian):**
```
offset  0: [6B] "ARXALP"
offset  6: [2B] version   (u16)
offset  8: [8B] manifest_len
offset 16: [8B] chunk_table_off
offset 24: [8B] chunk_count
offset 32: [8B] data_off
offset 40: [8B] flags
```

---

### LLD 2 — Manifest, ChunkTable & TailSummary Structs

`arx-core/src/container/manifest.rs`, `chunktab.rs`, `tail.rs`

```mermaid
classDiagram
    class Manifest {
        +files: Vec~FileEntry~
        +dirs: Vec~DirEntry~
        +meta: Meta
    }
    class FileEntry {
        +path: String
        +mode: u32
        +mtime: i64
        +u_size: u64
        +chunk_refs: Vec~ChunkRef~
    }
    class ChunkRef {
        +id: u64
        +u_size: u64
    }
    class DirEntry {
        +path: String
        +mode: u32
        +mtime: i64
    }
    class Meta {
        +created: i64
        +tool: String
    }
    class ChunkEntry {
        +codec: u8
        +u_size: u64
        +c_size: u64
        +data_off: u64
    }
    note for ChunkEntry "ENTRY_SIZE = 32 bytes\ncodec: 0=Store 1=Zstd\nc_size includes 16B AEAD tag if encrypted"
    class TailSummary {
        +manifest_blake3: [u8; 32]
        +chunktab_blake3: [u8; 32]
        +data_blake3: [u8; 32]
        +total_u: u64
        +total_c: u64
        +write_to(w: impl Write) Result~()~
        +read_from(r: impl Read) Result~TailSummary~
        +read_tail_at_eof(f: impl Read+Seek) Result~TailSummary~
    }
    note for TailSummary "TAIL_MAGIC = b\"ARXTAIL\\0\"\nTAIL_LEN = 120 bytes"

    Manifest "1" --> "*" FileEntry
    Manifest "1" --> "*" DirEntry
    Manifest "1" --> "1" Meta
    FileEntry "1" --> "*" ChunkRef
```

> `ChunkRef.id` is an index into the flat `Vec<ChunkEntry>` (the chunk table). Multiple `FileEntry` items may share the same `id` (deduplication).

---

### LLD 3 — Encryption Scheme

`arx-core/src/crypto/aead.rs`

```mermaid
flowchart TD
    A["AeadKey([u8; 32])\n32-byte raw key"] --> B["derive_nonce(\n  key_salt: &[u8;32],\n  region: Region,\n  chunk_id: u64\n)"]
    B --> C["blake3(\n  salt || region_byte || chunk_id_LE\n)\n→ take first 24 bytes\n→ XNonce for XChaCha20"]

    C --> D{"Operation"}
    D -->|Encrypt| E["seal_whole(\n  key, nonce, ad, plaintext\n)\n→ XChaCha20-Poly1305 encrypt\n→ ciphertext + 16-byte tag"]
    D -->|Decrypt| F["open_whole(\n  key, nonce, ad, ciphertext\n)\n→ verify 16-byte tag\n→ plaintext"]
```

**Region constants (domain separation):**

| Region | Value | Scope | Nonce chunk_id |
|--------|-------|-------|----------------|
| `Manifest` | 1 | Entire manifest CBOR | 0 |
| `ChunkTable` | 2 | Entire chunk table | 0 |
| `ChunkData` | 3 | Per individual chunk | chunk index |

**Associated data (ad) passed to AEAD:**
- Manifest: `b"manifest"`
- ChunkTable: `b"chunktab"`
- ChunkData: `b"chunk"`

**Size impact:** Every encrypted region grows by `TAG_LEN = 16` bytes.

---

### LLD 4 — FastCDC Content-Defined Chunking

`arx-core/src/chunking/fastcdc.rs`

```mermaid
classDiagram
    class ChunkParams {
        +min: usize
        +avg: usize
        +max: usize
    }
    note for ChunkParams "Default: min=65536 (64KiB)\navg=262144 (256KiB)\nmax=1048576 (1MiB)"

    class StreamingChunker {
        -p: ChunkParams
        -mask: u64
        -fp: u64
        -stash: Vec~u8~
        -pos: usize
        -scratch: Vec~u8~
        +new(p: ChunkParams) Self
        +next_chunk(r: impl Read, out: &mut Vec~u8~) Result~usize~
    }
    note for StreamingChunker "mask derived from avg\nfp = rolling Gear fingerprint\nnext_chunk returns 0 at EOF"

    StreamingChunker --> ChunkParams
```

**Boundary detection algorithm (inside `next_chunk`):**

```mermaid
flowchart TD
    A["Refill stash from reader\n(64 KiB scratch buffer)"] --> B["Reset fp = 0\nsize = 0"]
    B --> C["Read next byte"]
    C --> D["fp = (fp << 1) + GEAR[byte]\nsize += 1"]
    D --> E{"size >= max?"}
    E -->|Yes| F["Emit chunk\n(hard stop)"]
    E -->|No| G{"size >= min\nAND\n(fp & mask) == 0?"}
    G -->|Yes| F
    G -->|No| C
    F --> H["Return chunk bytes\n(0 = EOF)"]
```

The `GEAR` table is 256 × `u64` pseudo-random values built with SplitMix64, seeded deterministically. The `mask` is `(1 << bits) - 1` where `bits = ceil(log2(avg))`, giving a 1-in-avg probability of a boundary per byte.

---

### LLD 5 — Codec System

`arx-core/src/codec/`

```mermaid
classDiagram
    class CodecId {
        <<enumeration>>
        Store = 0
        Zstd = 1
    }

    class Compressor {
        <<trait>>
        +id() CodecId
        +compress(src: Read, dst: Write, level: i32) Result~u64~
        +decompress(src: Read, dst: Write) Result~u64~
    }

    class Store {
        +id() Store
        +compress() io::copy passthrough
        +decompress() io::copy passthrough
    }

    class ZstdCompressor {
        +id() Zstd
        +compress() zstd Encoder level 3
        +decompress() zstd Decoder
    }

    Compressor <|.. Store
    Compressor <|.. ZstdCompressor
```

**Factory function:**
```rust
// arx-core/src/codec/mod.rs
pub fn get_decoder_u8(codec: u8) -> Result<&'static dyn Compressor>
// Returns &Store (0) or &ZstdCompressor (1); errors on unknown codec
```

**Codec selection during pack:**
```
should_compress = (u_size - c_size) as f32 >= u_size as f32 * min_gain
```
where `min_gain` defaults to `0.05` (5% savings required).

---

### LLD 6 — Journal & Delta (CRUD On-Disk Format)

`arx-core/src/container/journal.rs`, `delta.rs`

```mermaid
classDiagram
    class EncMode {
        <<enumeration>>
        Plain
        Aead { key: [u8;32], salt: [u8;32] }
    }

    class Loc {
        <<enumeration>>
        Base
        Delta
    }

    class ChunkRef {
        +loc: Loc
        +off: u64
        +len: u64
        +codec: CodecId
        +blake3: [u8; 32]
    }

    class LogRecord {
        <<enumeration>>
        Put { path, mode, mtime, size, chunks: Vec~ChunkRef~ }
        Delete { path }
        Rename { from, to }
        SetPolicy(Policy)
        Note { text }
    }

    class Journal {
        -f: File
        -path: PathBuf
        -enc: EncMode
        -flags: u8
        -salt: [u8; 32]
        +open(path, enc) Result~Journal~
        +append(rec: LogRecord) Result~()~
        +iter() Result~JournalIter~
    }
    note for Journal "MAGIC = b\"ARXLOG\\0\\0\"\nVERSION = 1\nFLAG_AEAD = 0x01\nHeader = 41 bytes\nRecords: varint(len) + CBOR"

    class DeltaStore {
        -f: File
        -path: PathBuf
        -next_off: u64
        -enc: EncMode
        -salt: [u8; 32]
        +open(path, enc) Result~DeltaStore~
        +append_frame(plain: &[u8]) Result~(u64, u64)~
        +read_frame(off, len) Result~Box~dyn Read~~
    }
    note for DeltaStore "Returns (offset, length)\nFrames: varint(len) + bytes\nOptional AEAD per-frame"

    Journal --> LogRecord
    LogRecord --> ChunkRef
    ChunkRef --> Loc
    Journal --> EncMode
    DeltaStore --> EncMode
```

**Nonce derivation for sidecars:**

| Sidecar | Domain string | Extra input |
|---------|--------------|-------------|
| Journal record | `b"arxlog"` | `payload_off \|\| cipher_len` |
| Delta frame | `b"arxdelta"` | `payload_off \|\| cipher_len` |

Both derive 24-byte XChaCha20 nonces via `blake3(domain || salt || extra).take(24)`.

---

### LLD 7 — InMemIndex & CRUD State Machine

`arx-core/src/index/inmem.rs`, `arx-core/src/crud/mod.rs`

```mermaid
classDiagram
    class Entry {
        +mode: u32
        +mtime: u64
        +size: u64
        +chunks: Vec~ChunkRef~
    }

    class InMemIndex {
        +by_path: BTreeMap~String, Entry~
        +by_chunk: HashMap~[u8;32], (Loc, u64, u64, CodecId)~
        +policy: Policy
        +stats: Stats
        +from_base() Result~Self~
        +apply(rec: LogRecord)
    }
    note for InMemIndex "apply() dispatches on LogRecord variant\nDelete marks entry absent\nRename moves BTreeMap key"

    class CrudArchive {
        +base_path: PathBuf
        +log_path: PathBuf
        +delta_path: PathBuf
        +index: InMemIndex
        +journal: Journal
        +delta: DeltaStore
        +open(base) Result~Self~
        +open_with_crypto(base, key, salt) Result~Self~
        +put_file(src, dst, mode, mtime) Result~()~
        +delete_path(path) Result~()~
        +delete_path_recursive(path) Result~()~
        +rename(from, to) Result~()~
        +open_reader(path) Result~Box~dyn Read~~
        +sync_to_base(archive, out, ...) Result~()~
        +issue_archive(out, label, owner, notes, ...) Result~()~
    }

    CrudArchive --> InMemIndex
    CrudArchive --> Journal
    CrudArchive --> DeltaStore
    InMemIndex --> Entry
```

**State machine: opening a CrudArchive**

```mermaid
stateDiagram-v2
    [*] --> OpenFiles: open(base_path)
    OpenFiles --> ReadJournal: open journal sidecar\n(.arx.log)
    ReadJournal --> OpenDelta: open delta sidecar\n(.arx.delta)
    OpenDelta --> ReplayJournal: iterate LogRecords
    ReplayJournal --> ApplyPut: LogRecord::Put
    ReplayJournal --> ApplyDelete: LogRecord::Delete
    ReplayJournal --> ApplyRename: LogRecord::Rename
    ReplayJournal --> ApplyPolicy: LogRecord::SetPolicy
    ApplyPut --> ReplayJournal: upsert by_path + by_chunk
    ApplyDelete --> ReplayJournal: remove by_path entry
    ApplyRename --> ReplayJournal: move BTreeMap key
    ApplyPolicy --> ReplayJournal: update policy
    ReplayJournal --> Ready: EOF on journal
    Ready --> [*]
```

---

### LLD 8 — ArchiveRepo Trait & Reading Pipeline

`arx-core/src/repo.rs`, `repo_fs.rs`, `read/opened.rs`, `read/stream.rs`

```mermaid
classDiagram
    class ArchiveRepo {
        <<trait>>
        +list_files() Result~Vec~FileRow~~
        +chunk_map(path) Result~Vec~ChunkRow~~
        +open_reader(path) Result~Box~dyn Read~~
        +open_range(path, start, len) Result~Box~dyn Read~~
    }

    class OpenParams {
        +archive_path: PathBuf
        +aead_key: Option~[u8;32]~
        +key_salt: [u8; 32]
    }

    class FsArchiveRepo {
        -opened: Arc~Mutex~Opened~~
        +new(params: OpenParams) Result~Self~
        +list_files() ...
        +chunk_map(path) ...
        +open_reader(path) ...
        +open_range(path, start, len) ...
    }

    class Opened {
        -f: Arc~Mutex~File~~
        -sb: Superblock
        -manifest: Manifest
        -table: Vec~ChunkEntry~
        -aead: Option~(AeadKey, [u8;32])~
        -file_end_for_data: u64
        +open(path, key, salt) Result~Self~
        +list_entries() impl Iterator~FileEntry~
        +chunk_map_for(path) Result~Vec~ChunkView~~
        +open_reader(path) Result~FileReader~
        +open_range(path, start, len) Result~RangeReader~
    }

    class FileReader {
        -arx: Opened
        -chunk_ids: Vec~u32~
        -cur: usize
        -cur_buf: Option~Cursor~Vec~u8~~~
        +load_next() bool
    }
    note for FileReader "impl std::io::Read\nChunks loaded lazily on demand\nEach chunk: read → decrypt → decompress → buffer"

    class RangeReader {
        -inner: FileReader
        -remain: u64
    }
    note for RangeReader "impl std::io::Read\nConsumes start bytes upfront\nThen reads up to len bytes"

    ArchiveRepo <|.. FsArchiveRepo
    FsArchiveRepo --> Opened
    Opened --> FileReader
    Opened --> RangeReader
    FileReader --> RangeReader
```

**open_repo factory:**
```rust
// arx-core/src/repo_factory.rs
pub enum Backend { Fs }
pub fn open_repo(backend: Backend, p: OpenParams) -> Result<Box<dyn ArchiveRepo>>
```

---

### LLD 9 — Error Types

`arx-core/src/error.rs`

```mermaid
classDiagram
    class ArxError {
        <<enumeration>>
        Io(std::io::Error)
        Format(String)
    }
    note for ArxError "pub type Result~T~ = std::result::Result~T, ArxError~\nIo: from #[from] std::io::Error\nFormat: manual construction for validation failures"
```

**Where each variant surfaces:**

| Variant | Common causes |
|---------|---------------|
| `Io` | File not found, permission denied, unexpected EOF, disk full |
| `Format` | Bad CBOR, wrong magic bytes, tag mismatch (AEAD auth failure), path traversal attempt (`../`), mismatched sizes, unknown codec id, tail hash mismatch |

All public API functions return `Result<T>`. `arxdev` handlers propagate errors to the CLI top-level, which prints the error and exits non-zero.

---

## Key Constants Summary

| Constant | Value | Source |
|----------|-------|--------|
| `MAGIC` | `b"ARXALP"` | `container/superblock.rs` |
| `VERSION` | `3` | `container/superblock.rs` |
| `HEADER_LEN` | `48` bytes | `container/superblock.rs` |
| `FLAG_ENCRYPTED` | `1 << 0` | `container/superblock.rs` |
| `ENTRY_SIZE` | `32` bytes | `container/chunktab.rs` |
| `TAIL_MAGIC` | `b"ARXTAIL\0"` | `container/tail.rs` |
| `TAIL_LEN` | `120` bytes | `container/tail.rs` |
| `TAG_LEN` | `16` bytes | `crypto/aead.rs` |
| `JOURNAL_MAGIC` | `b"ARXLOG\0\0"` | `container/journal.rs` |
| `JOURNAL_VERSION` | `1` | `container/journal.rs` |
| `CDC min` | `65536` (64 KiB) | `chunking/fastcdc.rs` |
| `CDC avg` | `262144` (256 KiB) | `chunking/fastcdc.rs` |
| `CDC max` | `1048576` (1 MiB) | `chunking/fastcdc.rs` |
| `default min_gain` | `0.05` (5%) | `pack/writer.rs` |
| `Zstd level` | `3` | `codec/zstdc.rs` |
