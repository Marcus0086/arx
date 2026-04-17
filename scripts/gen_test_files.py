#!/usr/bin/env python3
"""
gen_test_files.py — generate test files of various types and sizes.

Usage:
  python3 scripts/gen_test_files.py [OPTIONS] OUTDIR

Options:
  --size SIZE      File size, e.g. 1KB 256MB 5GB (default: 1MB)
  --type TYPE      sparse | dense | random | text | image | all (default: all)
  --char CHAR      Character to use for dense/text modes (default: a)
  --count N        How many files of each type to generate (default: 1)

Examples:
  # 1 of each type at 1 MB into ./testfiles/
  python3 scripts/gen_test_files.py ./testfiles

  # 5 GB dense file of 'a'
  python3 scripts/gen_test_files.py --size 5GB --type dense --char a ./testfiles

  # 100 MB random file
  python3 scripts/gen_test_files.py --size 100MB --type random ./testfiles

  # 10 different 512KB files of each type
  python3 scripts/gen_test_files.py --size 512KB --type all --count 10 ./testfiles
"""

import argparse
import os
import random
import struct
import sys
import zlib


# ─── size parsing ────────────────────────────────────────────────────────────

def parse_size(s: str) -> int:
    s = s.strip().upper()
    units = {"B": 1, "KB": 1024, "MB": 1024**2, "GB": 1024**3, "TB": 1024**4}
    for suffix, mult in sorted(units.items(), key=lambda x: -len(x[0])):
        if s.endswith(suffix):
            return int(float(s[: -len(suffix)]) * mult)
    return int(s)


def fmt_size(n: int) -> str:
    for unit, mult in [("GB", 1024**3), ("MB", 1024**2), ("KB", 1024), ("B", 1)]:
        if n >= mult:
            val = n / mult
            return f"{val:.1f}{unit}" if val != int(val) else f"{int(val)}{unit}"
    return f"{n}B"


# ─── generators ──────────────────────────────────────────────────────────────

CHUNK = 4 * 1024 * 1024  # 4 MB write chunks


def write_sparse(path: str, size: int):
    """Mostly zeros — OS may or may not create a real sparse file."""
    with open(path, "wb") as f:
        f.seek(size - 1)
        f.write(b"\x00")


def write_dense(path: str, size: int, char: str):
    """Single repeated character (high compressibility)."""
    byte = char.encode()[0:1]
    buf = byte * min(CHUNK, size)
    with open(path, "wb") as f:
        remaining = size
        while remaining > 0:
            chunk = buf[: min(CHUNK, remaining)]
            f.write(chunk)
            remaining -= len(chunk)


def write_random(path: str, size: int):
    """Incompressible random bytes (urandom)."""
    with open(path, "wb") as f:
        remaining = size
        while remaining > 0:
            n = min(CHUNK, remaining)
            f.write(os.urandom(n))
            remaining -= n


def write_text(path: str, size: int, char: str):
    """Printable text lines — good middle ground compressibility."""
    words = [
        "the", "quick", "brown", "fox", "jumps", "over", "a", "lazy", "dog",
        "archive", "compress", "encrypt", "chunk", "manifest", "blake3", "zstd",
        "stream", "delta", "journal", "vault", "storage", "data", "bytes",
        char * 8,  # inject the requested char as a 'word'
    ]
    rng = random.Random(42)
    with open(path, "w", encoding="utf-8") as f:
        written = 0
        while written < size:
            line = " ".join(rng.choice(words) for _ in range(12)) + "\n"
            if written + len(line) > size:
                line = line[: size - written]
            f.write(line)
            written += len(line)


def _png_chunk(tag: bytes, data: bytes) -> bytes:
    crc = zlib.crc32(tag + data) & 0xFFFFFFFF
    return struct.pack(">I", len(data)) + tag + data + struct.pack(">I", crc)


def write_image(path: str, size: int):
    """
    Synthetic PNG. The image dimension is chosen so the raw pixel data is
    roughly `size` bytes when stored uncompressed. We use a noise pattern so
    compression doesn't shrink it too far below the requested size.
    """
    target_pixels = size // 3  # RGB
    side = max(1, int(target_pixels**0.5))
    width, height = side, side

    rng = random.Random(7)

    ihdr = struct.pack(">IIBBBBB", width, height, 8, 2, 0, 0, 0)  # 8-bit RGB

    # Build scanlines: filter byte 0x00 + RGB pixels (noisy)
    scanlines = bytearray()
    for _ in range(height):
        scanlines.append(0)  # filter type None
        row = bytes(rng.randint(0, 255) for _ in range(width * 3))
        scanlines.extend(row)

    idat_data = zlib.compress(bytes(scanlines), level=1)

    png = (
        b"\x89PNG\r\n\x1a\n"
        + _png_chunk(b"IHDR", ihdr)
        + _png_chunk(b"IDAT", idat_data)
        + _png_chunk(b"IEND", b"")
    )
    with open(path, "wb") as f:
        f.write(png)


# ─── dispatch ────────────────────────────────────────────────────────────────

GENERATORS = {
    "sparse": (write_sparse, ".bin"),
    "dense":  (write_dense,  ".bin"),
    "random": (write_random, ".bin"),
    "text":   (write_text,   ".txt"),
    "image":  (write_image,  ".png"),
}


def make_file(out_dir: str, type_name: str, size: int, char: str, index: int):
    fn, ext = GENERATORS[type_name]
    size_tag = fmt_size(size)
    name = f"{type_name}_{size_tag}_{index:03d}{ext}"
    path = os.path.join(out_dir, name)
    print(f"  {type_name:8s}  {size_tag:>8s}  →  {name}", end="", flush=True)
    if type_name in ("dense", "text"):
        fn(path, size, char)
    else:
        fn(path, size)
    actual = os.path.getsize(path)
    print(f"  (disk: {fmt_size(actual)})")
    return path


# ─── main ────────────────────────────────────────────────────────────────────

def main():
    p = argparse.ArgumentParser(
        description="Generate test files of various types and sizes.",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog=__doc__,
    )
    p.add_argument("outdir", metavar="OUTDIR", help="Output directory")
    p.add_argument("--size",  default="1MB", help="File size  (e.g. 1KB 256MB 5GB)")
    p.add_argument("--type",  default="all",
                   choices=list(GENERATORS) + ["all"],
                   help="Data pattern (default: all)")
    p.add_argument("--char",  default="a", help="Char for dense/text modes")
    p.add_argument("--count", type=int, default=1,
                   help="Files per type (default: 1)")
    args = p.parse_args()

    size = parse_size(args.size)
    os.makedirs(args.outdir, exist_ok=True)
    types = list(GENERATORS) if args.type == "all" else [args.type]

    print(f"\nGenerating {args.count}× each of {types} at {fmt_size(size)} into {args.outdir}/\n")

    total = 0
    for t in types:
        for i in range(args.count):
            make_file(args.outdir, t, size, args.char, i)
            total += size

    print(f"\nDone. Nominal total: {fmt_size(total)}")


if __name__ == "__main__":
    main()
