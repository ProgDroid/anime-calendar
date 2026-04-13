# Multi-stage build for the Rust backend server.
#
# Stage layout:
#   chef     — base image with build tools + cargo-chef installed
#   planner  — produces recipe.json (dependency fingerprint)
#   builder  — cooks deps from recipe (cached), then builds the binary
#   runtime  — minimal Debian image with just the binary

FROM rust:1.85-bookworm AS chef

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
FROM debian:bookworm-slim AS runtime

RUN apt-get update && \
    apt-get install -y --no-install-recommends ca-certificates libssl3 && \
    rm -rf /var/lib/apt/lists/*

RUN useradd --no-create-home --shell /bin/false appuser

WORKDIR /app

COPY --from=builder /app/target/release/server ./server

RUN chown appuser:appuser ./server

USER appuser

EXPOSE 8080

CMD ["./server"]
