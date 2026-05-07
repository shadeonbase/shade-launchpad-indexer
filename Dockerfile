# syntax=docker/dockerfile:1.7

# ─── stage 1: build ─────────────────────────────────────────────────────────
FROM rust:1.90-slim-bookworm AS build

RUN apt-get update && apt-get install -y --no-install-recommends \
        pkg-config libssl-dev librdkafka-dev ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /src

# Cache deps via cargo-chef-style hack: copy manifests first.
COPY Cargo.toml Cargo.lock rust-toolchain.toml ./
COPY crates/shade-indexer-core/Cargo.toml     crates/shade-indexer-core/Cargo.toml
COPY crates/shade-indexer-kafka/Cargo.toml    crates/shade-indexer-kafka/Cargo.toml
COPY crates/shade-indexer-enrich/Cargo.toml   crates/shade-indexer-enrich/Cargo.toml
COPY crates/shade-indexer-bytecode/Cargo.toml crates/shade-indexer-bytecode/Cargo.toml
COPY crates/shade-indexer-bin/Cargo.toml      crates/shade-indexer-bin/Cargo.toml

# Empty source files so cargo can resolve and prebuild deps.
RUN mkdir -p crates/shade-indexer-core/src \
             crates/shade-indexer-kafka/src \
             crates/shade-indexer-enrich/src \
             crates/shade-indexer-bytecode/src \
             crates/shade-indexer-bin/src \
    && echo 'fn main(){}' > crates/shade-indexer-bin/src/main.rs \
    && for c in shade-indexer-core shade-indexer-kafka shade-indexer-enrich shade-indexer-bytecode; do \
         echo '' > crates/$c/src/lib.rs ; \
       done

RUN cargo build --release -p shade-indexer-bin || true

# Now copy the real source and build for real.
COPY . .
RUN cargo build --release -p shade-indexer-bin

# ─── stage 2: runtime ───────────────────────────────────────────────────────
FROM debian:bookworm-slim AS runtime

RUN apt-get update && apt-get install -y --no-install-recommends \
        librdkafka1 libssl3 ca-certificates curl tini \
    && rm -rf /var/lib/apt/lists/*

# Run as non-root.
RUN groupadd -r shade && useradd -r -g shade -s /sbin/nologin shade

WORKDIR /app
COPY --from=build /src/target/release/shade-indexer /usr/local/bin/shade-indexer
COPY config     /app/config
COPY migrations /app/migrations

ENV SHADE_CONFIG=/app/config/indexer.toml \
    RUST_LOG=info,shade_indexer_core=debug

EXPOSE 9090 9091
USER shade

# tini reaps zombie children + handles SIGTERM cleanly for our graceful shutdown.
ENTRYPOINT ["/usr/bin/tini", "--", "/usr/local/bin/shade-indexer"]
CMD ["serve"]

HEALTHCHECK --interval=10s --timeout=3s --start-period=15s --retries=3 \
    CMD curl -fsS http://localhost:9091/readyz || exit 1
