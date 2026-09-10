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
| `--resume` | `false` | Resume a previous deployment from the local `.kora-deploy-state.json` file |
| `--no-cleanup-on-failure` | `false` | Do not close the buffer on failure, leaving it open so you can `--resume` later |

## Recovering from a failed deploy

When you start a fresh deployment, `kora-deploy` writes a `.kora-deploy-state.json`
file to your directory. This stores the program and buffer keypairs required to
deploy your code. If deployment succeeds, the file is automatically removed.
If it fails (e.g., timeout or rate limit), the file remains for recovery.

**Normal Resume Flow**:
Run the exact same `kora-deploy` command but add the `--resume` flag.
It reads `.kora-deploy-state.json`, skips already-written chunks,
and continues where it left off.

**Cleanup on Failure**:
By default, failed or interrupted deploys automatically close the buffer
to return rent to the paymaster. This prevents you from resuming.
If you anticipate connection issues, pass `--no-cleanup-on-failure`
on your initial run to leave the buffer open for `--resume`.

**Hash Mismatch**:
If you recompile your `.so` file before resuming, the hashes won't match.
`kora-deploy` will refuse to resume to prevent corrupted data.
Either restore the original `.so` file, or delete the state file
and start a fresh deploy.

**Manual Recovery / Deploy Timeout**:
Rarely, the final transaction might time out, leaving an ambiguous state.
You can check if the paymaster finalized it manually:
```bash
solana program show <PROGRAM_ID>
```
If it's not live, you can't resume, and automatic cleanup failed,
the buffer may be stuck. You can manually close it using
`solana program close`. The `.kora-deploy-state.json` file stores the
`program_keypair` and `buffer_keypair` as raw JSON byte arrays. To use them
with the Solana CLI, copy the array of numbers (e.g., `[1,2,...]`) into a new
file and pass that file to the CLI, then delete the state file to start over.

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
