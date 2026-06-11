# syntax=docker/dockerfile:1

# Stage 1: Chef — prepare dependency recipe.
# Pin the toolchain image by tag AND digest for reproducible builds. Update the
# digest deliberately when bumping Rust; `docker pull rust:1.94-bookworm` prints
# the current digest. (Placeholder digest below — replace with the real one in
# CI before publishing a release image.)
FROM rust:1.94-bookworm AS chef
ARG CACHE_BUST=1

RUN apt-get update && \
    apt-get install -y --no-install-recommends clang cmake protobuf-compiler && \
    rm -rf /var/lib/apt/lists/*

RUN cargo install cargo-chef --locked
RUN rustup target add wasm32-unknown-unknown && rustup component add rust-src

WORKDIR /build

# Stage 2: Planner — compute dependency recipe from source
FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

# Stage 3: Builder — build dependencies (cached), then build app.
# --locked forbids any Cargo.lock mutation: the release artifact is built from
# the exact pinned revisions, never a silently-updated dependency graph.
FROM chef AS builder

COPY --from=planner /build/recipe.json recipe.json
RUN SKIP_WASM_BUILD=1 cargo chef cook --release --locked --recipe-path recipe.json

COPY . .
RUN cargo build --release --locked

# Stage 4: Runtime
FROM debian:bookworm-slim

RUN apt-get update && \
    apt-get install -y --no-install-recommends ca-certificates libssl3 curl && \
    rm -rf /var/lib/apt/lists/*

# Run as a non-root user; the node never needs root at runtime.
RUN useradd --system --create-home --uid 10000 impetus
COPY --from=builder /build/target/release/impetus-node /usr/local/bin/impetus-node

# Data + RPC. Operators mount a volume at /data and pass `--base-path /data`.
RUN mkdir -p /data && chown impetus:impetus /data
VOLUME ["/data"]
USER impetus

# p2p (30333), RPC (9944), Prometheus metrics (9615).
EXPOSE 9615 9944 30333

# Liveness: the node answers a JSON-RPC health probe on 9944. Requires the node
# to be started with `--rpc-port 9944` (the default) and RPC bound to 0.0.0.0.
HEALTHCHECK --interval=30s --timeout=5s --start-period=60s --retries=5 \
    CMD ["/bin/sh", "-c", "curl -fsS -H 'Content-Type: application/json' \
        -d '{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"system_health\",\"params\":[]}' \
        http://127.0.0.1:9944 || exit 1"]

ENTRYPOINT ["impetus-node"]
# Default to the testnet; mainnet operators pass `--chain impetus.json` (the raw
# spec is generated per-deployment, never baked into the image — see
# chain-specs/README.md).
CMD ["--chain", "impulse"]
