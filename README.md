# ARX

**ARX is a modern archive format that acts like a compressed, encrypted object store in a single file.**  
It combines the simplicity of `.zip` with the power of S3: content-defined chunking, Zstd compression, per-region AEAD encryption, and append-only CRUD — all in one portable `.arx` file.

---

## What's in this repo

| Crate / Package | Description |
|---|---|
| `arx-core` | Rust library — all archive logic, zero unsafe code |
| `arxdev` | CLI binary (`arx`) — thin shell over arx-core |
| `arx-grpc` | Multi-tenant gRPC server with HTTP upload endpoint |
| `arxui` | Next.js 15 web UI — ARX Drive |
| `scripts/` | Developer utilities (test file generation, etc.) |

---

## arx-core

The portable archive library. No unsafe code (`#![forbid(unsafe_code)]`).

### On-disk format (v4)
```
[Superblock 80B] → [Manifest (CBOR)] → [Chunk Table] → [Chunk Data] → [Tail Summary]
```

- **Superblock** — magic `ARXALP`, version, offsets, `kdf_salt`
- **Manifest** — CBOR file/dir/symlink metadata
- **Chunk Table** — codec, sizes, data offset, blake3 hash per chunk
- **Tail Summary** — region-level blake3 integrity at EOF

### Key features

- **Streaming** — pack/extract TB-scale data without loading into RAM
- **FastCDC chunking** — content-defined, natural deduplication
- **Zstd compression** — high ratio with automatic STORE fallback
- **XChaCha20-Poly1305 encryption** — per-region AEAD, random access into encrypted archives
- **Append-only CRUD** — journal + delta sidecar, no rewrite of base archive
- **Deterministic mode** — reproducible builds for CI/CD
- **v3 backward compat** — read-only; v3 chunk entries have no blake3 integrity check

### Build & test

```bash
cargo build --release          # build all
cargo test --workspace         # run all tests
cargo test -p arx-core         # library tests only
cargo clippy && cargo fmt      # lint + format
```

### CLI quick reference

```bash
# Create / read
arx pack [--encrypt-raw KEY | --password PW] OUT INPUTS…
arx list ARCHIVE
arx extract ARCHIVE DEST
arx verify ARCHIVE

# CRUD overlay
arx crud add  ARCHIVE SRC DST
arx crud rm   ARCHIVE PATH
arx crud mv   ARCHIVE FROM TO
arx crud ls   ARCHIVE
arx crud diff ARCHIVE
arx crud sync ARCHIVE          # compact overlay into base (in-place)
arx crud cat  ARCHIVE PATH
arx crud get  ARCHIVE PATH OUT
```

---

## arx-grpc — gRPC server

Multi-tenant gRPC server (port 50051). Speaks gRPC-web so browsers connect directly.

### Architecture

- **Auth**: JWT access tokens (15 min) + refresh tokens (30 days, stored in DB). All RPCs require `Authorization: Bearer <token>`.
- **Storage**: `{ROOT_DIR}/tenants/{tenant_id}/archives/{archive_id}/data.arx` + CRUD sidecars + `meta.json` label sidecar
- **Upload**: Browser uploads via `POST /api/upload/{archive_id}` (multipart). Concurrent uploads transfer bytes in parallel; archive lock is only held during the commit phase.
- **Concurrency**: Per-archive `tokio::Mutex` prevents concurrent writes from corrupting the same archive.
- **Logging**: Structured `tracing` logs. Set `RUST_LOG=info` or `RUST_LOG=debug`.

### Running

```bash
# Development
ROOT_DIR=/tmp/arx-data ARX_ADMIN_KEY=devkey cargo run -p arx-grpc

# Environment variables
ROOT_DIR=/var/lib/arx       # data directory (default: ~/.arx-grpc)
PORT=50051                  # listen port
ARX_ADMIN_KEY=secret        # enables admin RPCs (CreateTenant, CreateUser, etc.)
CORS_ORIGIN=https://app.example.com  # restrict CORS; omit to allow all origins (dev)
RUST_LOG=info               # log level
```

### Endpoints

| Endpoint | Description |
|---|---|
| `GET /health` | Health check — returns `ok` |
| `POST /api/upload/{archive_id}` | Multipart file upload (auth via Bearer header) |
| `POST /arx.ArxService/*` | All gRPC-web RPCs |

### Admin setup (first run)

```bash
# Create a tenant + user
TOKEN=$(grpcurl -plaintext -d '{"email":"admin@example.com","password":"changeme"}' \
  localhost:50051 arx.ArxService/Login | jq -r .access_token)

# Or use the admin key for tenant/user creation
grpcurl -plaintext -H "authorization: Bearer $ARX_ADMIN_KEY" \
  -d '{"name":"my-org"}' localhost:50051 arx.ArxService/CreateTenant
```

---

## arxui — ARX Drive web UI

Next.js 15 app. Looks like a Google Drive for ARX vaults.

### Features

- **Vault management** — create, rename, delete vaults; inline rename
- **File browser** — virtualized grid (only ~20 DOM rows regardless of file count), lazy image thumbnails
- **Infinite scroll** — paginated file list (100/page) with IntersectionObserver sentinel
- **Concurrent uploads** — 3 parallel XHR uploads with per-file progress; folder drag-and-drop supported
- **File preview** — inline image, video, audio, text (first 64 KB), PDF (native browser viewer)
- **Vault stats** — logical vs stored bytes, compression ratio, savings bar
- **Right sidebar** — live clock, storage overview widget, recent activity widget
- **Session persistence** — refresh token in localStorage + cookie (30-day); access token in memory only
- **Error boundaries** — per-route error pages with retry
- **Toasts** — `sonner` for upload/download/delete/sync/rename failures

### Running

```bash
cd arxui
cp .env.example .env.local     # set NEXT_PUBLIC_ARX_URL
pnpm install
pnpm dev                       # http://localhost:3000
pnpm build                     # production build
```

### Environment

```bash
# arxui/.env.local
NEXT_PUBLIC_ARX_URL=http://localhost:50051   # arx-grpc server URL
```

### SDK

The TypeScript SDK lives in `arxui/src/sdk/`:

```ts
import { createSdk } from "@/src/sdk";

const sdk = createSdk({ baseUrl: "http://localhost:50051" });

// Auth
await sdk.auth.login(email, password);
await sdk.auth.logout();
const user = await sdk.auth.whoami();

// Vaults
const vaults = await sdk.vaults.list();
await sdk.vaults.create(name);
await sdk.vaults.rename(id, newName);
await sdk.vaults.delete(id);

// Files
const { entries, total } = await sdk.files.list(vaultId, { offset: 0, limit: 100 });
await sdk.files.uploadSingle(vaultId, { name, file }, onProgress);
const blob = await sdk.files.download(vaultId, path);
const { bytes, truncated } = await sdk.files.preview(vaultId, path, 64 * 1024);
await sdk.files.delete(vaultId, path);
await sdk.files.sync(vaultId);
```

---

## scripts/

### gen_test_files.py

Generate test files of various types and sizes for upload testing:

```bash
# One of each type at 1 MB
python3 scripts/gen_test_files.py ./testfiles

# 5 GB dense file (single repeated character — maximally compressible)
python3 scripts/gen_test_files.py --size 5GB --type dense --char a ./testfiles

# 500 MB incompressible random data
python3 scripts/gen_test_files.py --size 500MB --type random ./testfiles

# 10 × 256 KB of every type
python3 scripts/gen_test_files.py --size 256KB --count 10 --type all ./testfiles
```

Types: `sparse` (zeros), `dense` (repeated char), `random` (urandom), `text` (lorem-style), `image` (synthetic PNG)

---

## Security notes

- **Encryption**: XChaCha20-Poly1305, per-region. Wrong key → AEAD error, no data leaked.
- **Path sanitization**: Upload paths are sanitized server-side — `..` and absolute components are rejected with 400.
- **Tenant isolation**: All archive operations are scoped to the authenticated tenant; `archive_id` ownership is verified before any file access.
- **CORS**: Restrict to your frontend origin in production via `CORS_ORIGIN` env var.
- **TLS**: Terminate TLS at a reverse proxy (nginx, Caddy). The gRPC server speaks plain HTTP/1.1 internally.
- **Tokens**: Access tokens never written to disk (memory only). Refresh tokens use dual localStorage + `SameSite=Lax` cookie storage.

---

## Deployment (minimal)

```
┌─────────────┐    HTTPS/h2    ┌─────────────┐    HTTP/1.1    ┌──────────────┐
│   Browser   │ ─────────────▶ │  Caddy/nginx│ ─────────────▶ │  arx-grpc    │
└─────────────┘                │  TLS proxy  │                └──────────────┘
                                └─────────────┘
                                       │ static
                                       ▼
                                 Next.js (arxui)
```

```bash
# docker-compose example
NEXT_PUBLIC_ARX_URL=https://api.example.com \
ROOT_DIR=/data \
ARX_ADMIN_KEY=$(openssl rand -hex 32) \
CORS_ORIGIN=https://drive.example.com \
docker compose up
```

---

## License

MIT — see [LICENSE](LICENSE)
