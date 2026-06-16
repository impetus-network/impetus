# Railway image for an Impetus node. Builds the node binary from the repo, then
# wraps it with infra/railway/entrypoint.sh (loads node key + session keys from
# env, then starts validator/archive). Build context = repo root.
#
# Reuses the same multi-stage build as apps/node/Dockerfile to get impetus-node,
# then adds the entrypoint. Set Railway's "Dockerfile path" to
# infra/railway/Dockerfile and the build context to the repo root.

FROM rust:1.94-bookworm AS chef
ARG CACHE_BUST=1
# Fetch git deps (polkadot-sdk) via the git CLI, not libgit2: libgit2's TLS is
# flaky under cross-arch emulation (SSL class=Ssl errors). Retry transient fails.
ENV CARGO_NET_GIT_FETCH_WITH_CLI=true CARGO_NET_RETRY=10
RUN apt-get update && \
    apt-get install -y --no-install-recommends clang cmake protobuf-compiler git && \
    rm -rf /var/lib/apt/lists/*
RUN cargo install cargo-chef --locked
RUN rustup target add wasm32-unknown-unknown && rustup component add rust-src
WORKDIR /build

FROM chef AS planner
COPY apps/node/ .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
COPY --from=planner /build/recipe.json recipe.json
RUN SKIP_WASM_BUILD=1 cargo chef cook --release --locked --recipe-path recipe.json
COPY apps/node/ .
RUN cargo build --release --locked

FROM debian:bookworm-slim
RUN apt-get update && \
    apt-get install -y --no-install-recommends ca-certificates libssl3 curl && \
    rm -rf /var/lib/apt/lists/*

RUN useradd --system --create-home --uid 10000 impetus
COPY --from=builder /build/target/release/impetus-node /usr/local/bin/impetus-node
COPY infra/railway/entrypoint.sh /usr/local/bin/entrypoint.sh
RUN chmod +x /usr/local/bin/entrypoint.sh

RUN mkdir -p /data && chown impetus:impetus /data
VOLUME ["/data"]
USER impetus

# p2p, RPC, Prometheus metrics.
EXPOSE 9615 9944 30333

HEALTHCHECK --interval=30s --timeout=5s --start-period=120s --retries=5 \
    CMD ["/bin/sh", "-c", "curl -fsS -H 'Content-Type: application/json' \
        -d '{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"system_health\",\"params\":[]}' \
        http://127.0.0.1:9944 || exit 1"]

ENTRYPOINT ["/usr/local/bin/entrypoint.sh"]
