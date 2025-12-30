# Build stage
FROM rust:1.85-slim AS builder

RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    cmake \
    git \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /usr/src/git-versioner
COPY . .

RUN cargo build --release

# Final stage
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y \
    libssl3 \
    ca-certificates \
    git \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /usr/src/git-versioner/target/release/git-versioner /usr/local/bin/git-versioner
COPY entrypoint.sh /entrypoint.sh
RUN chmod +x /entrypoint.sh

ENTRYPOINT ["/entrypoint.sh"]
