# Stage 1: Build the Rust binary
FROM rust:1.93-slim AS builder
WORKDIR /build
COPY Cargo.toml Cargo.lock ./
COPY crates/ crates/
COPY docs/ docs/
RUN cargo build --release -p tenor-cli

# Stage 2: Minimal runtime image
FROM debian:trixie-slim
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /build/target/release/tenor /usr/local/bin/tenor

# Default port
EXPOSE 8080

# Contracts are mounted at /contracts
VOLUME ["/contracts"]

ENTRYPOINT ["tenor", "serve", "--port", "8080"]
# Users can append contract paths: docker run tenor/evaluator /contracts/my_contract.tenor
