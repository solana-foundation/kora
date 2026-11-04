: "${KORA_API_KEY:?KORA_API_KEY must be set}"
kora --config kora.toml --rpc-url https://api.mainnet.solana.com rpc start --signers-config signers.toml
