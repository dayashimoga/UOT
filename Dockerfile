# Multi-stage Docker build for Universal Offline Transfer (UOT) Rust Engine
FROM rust:1.80-slim as builder

WORKDIR /app
COPY rust /app/rust

WORKDIR /app/rust
RUN cargo test --no-run

FROM rust:1.80-slim

RUN apt-get update && apt-get install -y \
    net-tools \
    iproute2 \
    iputils-ping \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY --from=builder /app /app

EXPOSE 42000 5353/udp

CMD ["cargo", "test", "--manifest-path", "rust/Cargo.toml", "--", "--nocapture"]
