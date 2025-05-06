# Builder stage
FROM rust:latest AS builder

# Install specific nightly toolchain
RUN rustup default nightly-2025-05-04
RUN rustup component add clippy rustfmt

# Install required system dependencies
RUN apt-get update && apt-get install -y \
  pkg-config \
  libssl-dev \
  && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy dependency specifications
COPY Cargo.toml Cargo.lock ./

# Create dummy source to cache dependencies
RUN mkdir src && \
  echo "fn main() {}" > src/main.rs && \
  cargo build --release && \
  rm -rf target/release/.fingerprint/good-first-bot-rs

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
