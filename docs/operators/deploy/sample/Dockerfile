FROM rust:1.87

# Install kora-rpc from crates.io
RUN cargo install kora-cli

# Copy config
COPY kora.toml ./

# Copy signers.toml
COPY signers.toml ./

# Run the installed kora-rpc binary (it's in PATH after cargo install)
CMD kora rpc --config kora.toml start --signers-config signers.toml --port ${PORT:-8080} --rpc-url ${RPC_URL}