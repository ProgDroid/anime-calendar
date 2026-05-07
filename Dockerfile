# Multi-stage build for the Rust backend server.
#
# Stage layout:
#   chef     — base image with build tools + cargo-chef installed
#   planner  — produces recipe.json (dependency fingerprint)
#   builder  — cooks deps from recipe (cached), then builds the binary
#   runtime  — minimal Debian image with just the binary

# Rust 1.95.0 is current stable as of 2026-05; bumped from 1.85 for ~6 months
# of CVE patches in libssl3/glibc/ca-certificates layers (AUDIT M-19).
# Pin patch version explicitly — never use a rolling minor tag in CI/CD.
FROM rust:1.95.0-bookworm AS chef

# nasm + cmake are required by aws-lc-sys (the JWT cryptography backend)
RUN apt-get update && \
    apt-get install -y --no-install-recommends nasm cmake && \
    rm -rf /var/lib/apt/lists/*

RUN cargo install cargo-chef --locked

WORKDIR /app

# ---------------------------------------------------------------------------
FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

# ---------------------------------------------------------------------------
FROM chef AS builder

# Cook dependencies first — this layer is cached until Cargo.toml/Cargo.lock changes
COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json

# Copy source and the sqlx offline query cache
COPY . .

# Use the committed .sqlx/ cache so no live database is needed at build time.
# Run `cargo sqlx prepare --workspace` locally and commit .sqlx/ after any query change.
ENV SQLX_OFFLINE=true

RUN cargo build --release --package server

# ---------------------------------------------------------------------------
# Pinned to digest for supply-chain reproducibility (AUDIT M-20).
# To bump: docker pull debian:bookworm-slim && docker inspect debian:bookworm-slim | jq '.[0].RepoDigests'
# Digest last refreshed: 2026-05-07 (bookworm-slim pushed ~16 days prior).
FROM debian:bookworm-slim@sha256:5a2a80d11944804c01b8619bc967e31801ec39bf3257ab80b91070eb23625644 AS runtime

# curl   — required by the HEALTHCHECK probe below.
# tini   — PID-1 signal reaper; ensures SIGTERM from `docker stop` reaches the
#          server process instead of being silently dropped by an unhandled PID-1.
# (AUDIT M-20 — both additions)
RUN apt-get update && \
    apt-get install -y --no-install-recommends ca-certificates libssl3 curl tini && \
    rm -rf /var/lib/apt/lists/*

RUN useradd --no-create-home --shell /bin/false appuser

WORKDIR /app

COPY --from=builder /app/target/release/server ./server

RUN chown appuser:appuser ./server

USER appuser

EXPOSE 8080
# Note: the backend listens on port 8080 (hardcoded in config.toml.dist defaults).
# If the listen port ever changes, update the HEALTHCHECK URL below accordingly.

# Health check: probe /public-config — a public, unauthenticated bootstrap
# endpoint served by the backend at this internal path. nginx strips the /api
# prefix externally, so inside the container the path is /public-config.
# --fail  → curl exits non-zero on HTTP 4xx/5xx, so the HEALTHCHECK reflects
#            backend liveness, not just TCP openness.
# Phase 3 follow-up: add a dedicated /health endpoint reporting DB+Redis state.
HEALTHCHECK --interval=30s --timeout=5s --start-period=10s --retries=3 \
    CMD curl --fail --silent --show-error http://localhost:8080/public-config || exit 1

# tini is the ENTRYPOINT so it becomes PID 1 and proxies signals to ./server.
# CMD remains the application command; `--` separates tini args from the child.
ENTRYPOINT ["/usr/bin/tini", "--"]
CMD ["./server"]
