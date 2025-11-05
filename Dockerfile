# --- Stage 1: Base image with Rust toolchain and cargo-chef installed ---
FROM rust:1.85-alpine AS chef
USER root

# Install system dependencies
RUN apk add --no-cache musl-dev openssl-dev pkgconfig build-base

# Install cargo-chef to enable dependency caching
RUN cargo install cargo-chef

WORKDIR /app

# --- Stage 2: Dependency planner ---
FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

# --- Stage 3: Build dependencies and app ---
FROM chef AS builder
COPY --from=planner /app/recipe.json recipe.json

# Build dependencies (cached)
RUN cargo chef cook --release --recipe-path recipe.json

# Copy source code and build the actual binary
COPY . .
RUN cargo build --release --bin arb-bot

# --- Stage 4: Create minimal runtime image ---
FROM debian:bookworm-slim AS runtime
WORKDIR /app

# Install minimal runtime deps (CA certs for RPC HTTPS)
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates && rm -rf /var/lib/apt/lists/*

# Copy compiled binary
COPY --from=builder /app/target/release/arb-bot /usr/local/bin/arb-bot

# Create non-root user for safety
RUN useradd -m arbuser
USER arbuser

ENTRYPOINT ["/usr/local/bin/arb-bot"]
