FROM rust:1.86-slim AS builder

RUN apt-get update && apt-get install -y pkg-config libssl-dev && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# 先复制依赖清单，利用 Docker 缓存
COPY Cargo.toml Cargo.lock ./
COPY shared/ shared/
COPY cloud-relay/Cargo.toml cloud-relay/

# 创建 placeholder 源码以预编译依赖
RUN mkdir -p cloud-relay/src && echo "fn main() {}" > cloud-relay/src/main.rs && \
    cargo build --release -p cloud-relay 2>/dev/null || true && \
    rm -rf cloud-relay/src

# 复制真实源码并编译
COPY cloud-relay/src cloud-relay/src
RUN touch cloud-relay/src/main.rs && cargo build --release -p cloud-relay

# --- Runtime ---
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates && \
    rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/cloud-relay /usr/local/bin/cloud-relay

WORKDIR /data
EXPOSE 9800

CMD ["cloud-relay"]
