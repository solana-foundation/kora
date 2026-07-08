# kora-deploy

Deploy and upgrade Solana programs through a Kora paymaster without holding SOL.

The Kora paymaster pays buffer + program rent, signs every transaction as fee
payer, and keeps the on-chain upgrade authority. Your wallet is recorded in a
deploy registry at deploy time; only its signature can upgrade or close the
program through the paymaster. Programs idle for 7+ days are closed
automatically and the rent returns to the paymaster.

## Install

```bash
cargo install kora-deploy
```

## Use

```bash
kora-deploy --program-so ./my-program.so
```

Defaults to `https://deployer.devnet.solana.com`. Override with `--kora-url`.

The deploy prints the new program id. To upgrade later, pass it back with
`--program-id`, signed by the same wallet you deployed with. Omitting
`--program-id` always deploys a fresh program.

Flags:

| Flag | Default | Notes |
| --- | --- | --- |
| `--kora-url` | `https://deployer.devnet.solana.com` | Paymaster endpoint |
| `--rpc-url` | `https://api.devnet.solana.com` | Solana RPC for reads |
| `--program-so` | _required_ | Path to your `.so` |
| `--user-id` | random per run | Tag the paymaster buckets by for usage limits |
| `--wallet` | `~/.config/solana/id.json` | Owner wallet registered for upgrades; without one the program is immutable |
| `--program-id` | _(none)_ | Existing program to upgrade; omit to deploy fresh |

## Trade-offs

- You don't pay anything.
- You don't own the on-chain upgrade authority — the paymaster does. Upgrades
  and closes go through the paymaster, gated on your registered wallet's
  signature.
- Deploying without a wallet makes the program immutable — nobody can upgrade
  or close it through the paymaster; it just waits for the reaper.
- The program gets reaped after 7 days of on-chain idleness.

Production deploys should go to a paid RPC. This is for devnet.

## Source

The full source lives in
[`solana-foundation/kora`](https://github.com/solana-foundation/kora) under
`crates/kora-deploy/`.
