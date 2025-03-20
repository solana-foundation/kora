FROM rust:1.70 AS builder

WORKDIR /usr/src/app
COPY . .
RUN cargo build --release

FROM debian:bullseye-slim
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /usr/src/app/target/release/kora-rpc /usr/local/bin/

# Create configuration directory
RUN mkdir -p /etc/kora
# Copy default configuration
COPY kora.toml /etc/kora/kora.toml

# Environment variables should be provided at runtime
# Example: docker run -e RPC_URL=https://api.devnet.solana.com ...

# To override the config file at runtime, you can:
# 1. Mount a volume: docker run -v /path/to/your/kora.toml:/etc/kora/kora.toml ...
# 2. Mount to XDG config: docker run -v /path/to/your/config:/root/.config/kora ...
# 3. Use --config flag: docker run --config /path/to/your/kora.toml ...

# Default port for the RPC server (can be overridden with -p flag)
EXPOSE 8080
ENTRYPOINT ["kora-rpc"]
