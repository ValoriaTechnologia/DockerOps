# Stage 1: Build
FROM rust:1-bookworm AS builder

RUN apt-get update && apt-get install -y --no-install-recommends \
    libgit2-dev \
    libssl-dev \
    pkg-config \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Cache dependencies
COPY Cargo.toml Cargo.lock* ./
RUN mkdir src && echo "fn main() {}" > src/main.rs && cargo build --release && rm -rf src

# Build application
COPY src ./src
RUN cargo build --release

# Stage 2: Runtime
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    docker.io \
    && rm -rf /var/lib/apt/lists/*

RUN mkdir -p /data

ENV DOCKEROPS_DB_PATH=/data/dockerops.db

COPY --from=builder /app/target/release/dockerops /usr/local/bin/dockerops

# Optional: when running in Swarm with secret mounted at /run/secrets/github_token, export GITHUB_TOKEN
RUN echo '#!/bin/sh\n\
if [ -f /run/secrets/github_token ]; then\n\
  export GITHUB_TOKEN=$(cat /run/secrets/github_token)\n\
fi\n\
exec dockerops "$@"' > /entrypoint.sh && chmod +x /entrypoint.sh

ENTRYPOINT ["/entrypoint.sh"]
CMD ["run"]
