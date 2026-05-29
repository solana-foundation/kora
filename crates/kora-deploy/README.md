# kora-deploy

Deploy Solana programs through a Kora paymaster without holding SOL.

The Kora paymaster pays buffer + program rent, signs every transaction as fee
payer, and keeps upgrade authority on the deployed program. Programs idle for
7+ days are closed automatically and the rent returns to the paymaster.

## Install

```bash
cargo install kora-deploy
```

## Use

```bash
kora-deploy --program-so ./my-program.so
```

Defaults to `https://deployer.devnet.solana.com`. Override with `--kora-url`.

Flags:

| Flag | Default | Notes |
| --- | --- | --- |
| `--kora-url` | `https://deployer.devnet.solana.com` | Paymaster endpoint |
| `--rpc-url` | `https://api.devnet.solana.com` | Solana RPC for reads |
| `--program-so` | _required_ | Path to your `.so` |
| `--user-id` | `kora-deploy` | Tag the paymaster buckets by for usage limits |

## Trade-offs

- You don't pay anything.
- You don't own upgrade authority — the paymaster does.
- You can't close the program — it gets reaped after 7 days of on-chain idleness.

Production deploys should go to a paid RPC. This is for devnet.

## Source

The full source lives in
[`solana-foundation/kora`](https://github.com/solana-foundation/kora) under
`crates/kora-deploy/`.
