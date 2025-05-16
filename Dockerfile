# ---- Chef Stage ----
FROM rust:1.77-bullseye AS chef 

RUN rustup update && \
  rustup toolchain install stable --profile minimal --component cargo --component rustc && \
  rustup default stable
RUN cargo install cargo-chef

RUN rustup toolchain install nightly-2025-05-04 --profile minimal --component cargo --component rustc --component rust-std && \
  rustup default nightly-2025-05-04

# ---- Planner Stage ----
FROM chef AS planner
WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY .sqlx ./.sqlx 
RUN mkdir src && echo "fn main() {println!(\"Planner dummy main\");}" > src/main.rs

RUN cargo chef prepare --recipe-path recipe.json

# ---- Builder Stage ----
FROM chef AS builder

RUN apt-get update && apt-get install -y pkg-config libssl-dev \
  && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY --from=planner /app/recipe.json recipe.json
COPY .sqlx ./.sqlx

RUN cargo chef cook --release --recipe-path recipe.json

COPY src ./src
COPY migrations ./migrations
COPY Cargo.toml Cargo.lock ./

RUN cargo build --release --locked

# ---- Runtime Stage ----
FROM gcr.io/distroless/cc-debian11 AS runtime 

WORKDIR /app
COPY --from=builder /app/target/release/good-first-bot-rs /app/ 
COPY --from=builder /app/migrations ./migrations

ENTRYPOINT ["/app/good-first-bot-rs"]
