FROM rust:1.87

# Install kora-rpc from crates.io
RUN cargo install kora-rpc

# Copy config
COPY kora.toml ./

# Run the installed kora-rpc binary (it's in PATH after cargo install)
CMD kora-rpc --port ${PORT:-8080} --rpc-url ${RPC_URL} --private-key ${KORA_PRIVATE_KEY}