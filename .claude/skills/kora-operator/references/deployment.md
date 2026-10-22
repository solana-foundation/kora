# Deployment

Both config files must be present in the container and every secret must arrive through the
environment, never baked into the image. `signers.toml` names env vars; it does not hold keys.

## Docker

Pre-built images are published to `ghcr.io/solana-foundation/kora` with tags `latest`, `beta`, and
`v<version>`. Pin `v<version>` in production — `latest` tracks stable releases and will move under
a running deployment on the next publish.

```bash
docker run \
  -v ./kora.toml:/kora.toml \
  -v ./signers.toml:/signers.toml \
  -e RPC_URL=https://api.mainnet-beta.solana.com \
  -e KORA_PRIVATE_KEY=<key> \
  -p 8080:8080 \
  ghcr.io/solana-foundation/kora:v<version>
```

Building from source instead:

```dockerfile
FROM rust:1.86-bookworm AS builder
RUN cargo install kora-cli

FROM debian:bookworm-slim
COPY --from=builder /usr/local/cargo/bin/kora /usr/local/bin/kora
COPY kora.toml signers.toml ./
ENV RPC_URL=https://api.mainnet-beta.solana.com
CMD ["kora", "rpc", "start", "--signers-config", "signers.toml"]
```

## Railway

1. Create a project containing `kora.toml`, `signers.toml`, and a Dockerfile.
2. `railway login && railway init && railway up`
3. Set `RPC_URL`, `KORA_PRIVATE_KEY`, and `RUST_LOG` in the dashboard.
4. Generate a public domain from project settings.

Full walkthrough: https://launch.solana.com/docs/kora/operators/deployment/railway

## Before going live

- `kora config validate-with-rpc` against the production RPC endpoint, not just `config validate` —
  it catches an unfunded fee payer and non-existent mints.
- Run `rpc initialize-atas` if the node accepts token payments.
- Point a health check at `/liveness`; it bypasses auth by design.
- A public node with no `api_key` or `hmac_secret` set is an open paymaster. Rate limiting alone
  does not stop a funded attacker draining the fee payer.
