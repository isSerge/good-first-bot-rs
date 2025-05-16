# Builder stage
FROM rust:latest AS builder

# Install specific nightly toolchain
RUN rustup default nightly-2025-05-04
RUN rustup component add clippy rustfmt
RUN rustup target add aarch64-unknown-linux-musl

# Install required system dependencies
RUN apt-get update && apt-get install -y \
  pkg-config \
  libssl-dev \
  && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy dependency specifications
COPY Cargo.toml Cargo.lock ./

# Build *only* dependencies
RUN cargo build --release --package non_existent_package_to_build_only_deps || true 

# Copy actual source code
COPY src ./src
COPY migrations ./migrations

# Build the application
RUN cargo build --release

# Runtime stage
FROM debian:bullseye-slim

RUN apt-get update && apt-get install -y \
  openssl \
  ca-certificates \
  && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy binary and resources from builder
COPY --from=builder /app/target/release/good-first-bot-rs /app/
COPY --from=builder /app/migrations /app/migrations

# Create data directory for SQLite
RUN mkdir -p /data

# Set environment variables
ENV DATABASE_URL=sqlite:///data/data.db
ENV RUST_LOG=info

# Expose port (adjust based on your application)
EXPOSE 8080

# Set entrypoint
ENTRYPOINT ["/app/good-first-bot-rs"]
