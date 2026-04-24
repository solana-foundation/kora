FROM rustlang/rust:nightly

# Build from main until a crates.io release includes the DeployAuthority plugin and
# KORA_REDIS_URL support. Switch back to `cargo install kora-cli` once that lands.
# The Kora workspace pins nightly via rust-toolchain.toml; matching here avoids
# dep-version skew (e.g. aws-sdk-kms currently requires rustc 1.91.1+).
RUN cargo install \
    --git https://github.com/solana-foundation/kora \
    --branch main \
    kora-cli

COPY kora.toml ./
COPY signers.toml ./

CMD kora --config kora.toml --rpc-url ${RPC_URL} rpc start --signers-config signers.toml --port ${PORT:-8080}
