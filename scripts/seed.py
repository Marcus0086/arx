#!/usr/bin/env python3
"""
Seed arx-grpc: create first tenant + user via grpc-web (HTTP/1.1, stdlib only).

Usage (inside the container or against a running server):
  python3 /usr/local/bin/seed.py

Environment:
  ARX_URL          server base URL  (default: http://localhost:50051)
  ARX_ADMIN_KEY    required
  SEED_TENANT      tenant name      (default: main)
  SEED_EMAIL       required
  SEED_PASSWORD    required
"""
import os, struct, sys, urllib.request, urllib.error

ARX_URL      = os.environ.get("ARX_URL",       "http://localhost:50051")
ADMIN_KEY    = os.environ.get("ARX_ADMIN_KEY",  "")
TENANT_NAME  = os.environ.get("SEED_TENANT",   "main")
EMAIL        = os.environ.get("SEED_EMAIL",     "")
PASSWORD     = os.environ.get("SEED_PASSWORD",  "")

def die(msg):
    print(f"error: {msg}", file=sys.stderr)
    sys.exit(1)

if not ADMIN_KEY: die("ARX_ADMIN_KEY not set")
if not EMAIL:     die("SEED_EMAIL not set")
if not PASSWORD:  die("SEED_PASSWORD not set")

# ── minimal protobuf encode/decode (string fields only) ───────────────────────

def _varint(n):
    out = []
    while n > 0x7F:
        out.append((n & 0x7F) | 0x80)
        n >>= 7
    out.append(n)
    return bytes(out)

def _field(num, value):
    enc = value.encode()
    return _varint((num << 3) | 2) + _varint(len(enc)) + enc

def encode_proto(*pairs):
    return b"".join(_field(n, v) for n, v in pairs)

def decode_proto(data):
    out, i = {}, 0
    while i < len(data):
        tag, shift = 0, 0
        while i < len(data):
            b = data[i]; i += 1
            tag |= (b & 0x7F) << shift
            if not (b & 0x80): break
            shift += 7
        wire = tag & 7
        num  = tag >> 3
        if wire == 2:
            length, shift2 = 0, 0
            while i < len(data):
                b = data[i]; i += 1
                length |= (b & 0x7F) << shift2
                if not (b & 0x80): break
                shift2 += 7
            out[num] = data[i:i + length].decode(errors="replace")
            i += length
        else:
            break
    return out

# ── grpc-web call ─────────────────────────────────────────────────────────────

def rpc(path, proto_bytes):
    frame = b"\x00" + struct.pack(">I", len(proto_bytes)) + proto_bytes
    req = urllib.request.Request(f"{ARX_URL}{path}", data=frame, method="POST")
    req.add_header("content-type", "application/grpc-web+proto")
    req.add_header("authorization", f"Bearer {ADMIN_KEY}")
    try:
        with urllib.request.urlopen(req, timeout=15) as r:
            raw = r.read()
    except urllib.error.HTTPError as e:
        die(f"HTTP {e.code}: {e.read().decode(errors='replace')}")
    # strip grpc-web 5-byte data-frame header
    return decode_proto(raw[5:]) if len(raw) > 5 else {}

# ── seed ──────────────────────────────────────────────────────────────────────

print(f"Creating tenant '{TENANT_NAME}'...")
t = rpc("/arx.ArxService/CreateTenant", encode_proto((1, TENANT_NAME)))
if t.get(2): die(f"CreateTenant failed: {t[2]}")
tenant_id = t.get(1, "")
if not tenant_id: die("no tenant_id in response — check ARX_ADMIN_KEY")
print(f"  tenant_id: {tenant_id}")

print(f"Creating user '{EMAIL}'...")
u = rpc("/arx.ArxService/CreateUser",
        encode_proto((1, tenant_id), (2, EMAIL), (3, PASSWORD)))
if u.get(2): die(f"CreateUser failed: {u[2]}")
print(f"  user_id:   {u.get(1, '?')}")

print(f"\nDone. Log in with: {EMAIL}")
