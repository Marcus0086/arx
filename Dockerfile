# syntax=docker/dockerfile:1.7
# ─── ARX Archive Service ───────────────────────────────────────────────────────
# Multi-stage Rust build with cargo-chef dependency caching.
#
# Targets:
#   (default) server  — production gRPC server
#   cli               — arx command-line tool
#
# Build:
#   docker build -t arx-grpc:latest .
#   docker build --target cli -t arx-cli:latest .

# ── Base: Rust + system deps + cargo-chef ─────────────────────────────────────
FROM rust:1-slim-bookworm AS chef
# gcc + libc6-dev required by zstd-sys (compiles C sources via cc crate)
RUN apt-get update && apt-get install -y --no-install-recommends \
        gcc \
        libc6-dev \
        pkg-config \
    && rm -rf /var/lib/apt/lists/*
RUN cargo install cargo-chef --locked
WORKDIR /build

# ── Planner: analyse Cargo workspace and emit a dependency recipe ─────────────
FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

# ── Builder: compile deps (cached), then compile our code ────────────────────
FROM chef AS builder

# Step 1 — build deps only.
# This Docker layer is invalidated only when Cargo.lock changes, so a code-only
# change skips the expensive dependency compilation entirely.
COPY --from=planner /build/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json

# Step 2 — build application binaries on top of the cached deps.
COPY . .
RUN cargo build --release -p arx-grpc -p arxdev

# ── CLI image ─────────────────────────────────────────────────────────────────
FROM debian:bookworm-slim AS cli
RUN apt-get update && apt-get install -y --no-install-recommends \
        ca-certificates \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /build/target/release/arxdev /usr/local/bin/arx

WORKDIR /data
ENTRYPOINT ["/usr/local/bin/arx"]

# ── Server image (default target) ─────────────────────────────────────────────
FROM debian:bookworm-slim AS server
RUN apt-get update && apt-get install -y --no-install-recommends \
        ca-certificates \
        wget \
        python3 \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /build/target/release/arx-grpc /usr/local/bin/arx-grpc
COPY scripts/seed.py /usr/local/bin/seed.py
RUN chmod +x /usr/local/bin/seed.py

RUN mkdir -p /data /etc/arx

# Runtime configuration — all overridable via environment or compose
ENV ROOT_DIR=/data \
    PORT=50051

EXPOSE 50051

HEALTHCHECK --interval=30s --timeout=5s --start-period=15s --retries=3 \
    CMD wget -qO- http://localhost:${PORT}/health || exit 1

ENTRYPOINT ["/usr/local/bin/arx-grpc"]
