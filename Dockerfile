# Docker Multi-Device E2E Test Infrastructure
#
# Simulates real multi-device scenarios: two isolated UOT nodes on a bridge
# network performing discovery, key exchange, and file transfer.

FROM rust:1.80-slim AS builder

RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY rust /app/rust

WORKDIR /app/rust
RUN cargo build --release --tests 2>&1 || cargo build --tests 2>&1

# Runtime stage
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y \
    net-tools \
    iproute2 \
    iputils-ping \
    ca-certificates \
    openssl \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY --from=builder /app /app
COPY --from=builder /usr/local/rustup /usr/local/rustup
COPY --from=builder /usr/local/cargo /usr/local/cargo

ENV PATH="/usr/local/cargo/bin:${PATH}"
ENV RUSTUP_HOME="/usr/local/rustup"
ENV CARGO_HOME="/usr/local/cargo"

EXPOSE 42000 5353/udp

# Default: run all tests including E2E and load tests
CMD ["cargo", "test", "--manifest-path", "rust/Cargo.toml", "--", "--nocapture", "--test-threads=2"]
