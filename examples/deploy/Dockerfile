FROM rust:1.88

# Install kora-rpc from crates.io
RUN cargo install kora-cli

# Copy config
COPY kora.toml ./

# Copy signers.toml
COPY signers.toml ./

# Run the installed kora-rpc binary (it's in PATH after cargo install)
CMD kora --config kora.toml  --rpc-url ${RPC_URL} rpc start --signers-config signers.toml --port ${PORT:-8080}
