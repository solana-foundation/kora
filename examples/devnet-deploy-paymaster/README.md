# Devnet Deploy Paymaster

A Kora paymaster that sponsors program deploys on Solana devnet so devs don't
have to drip-faucet several SOL. Upgrade authority is pinned to the paymaster
so the SOL can't be drained; programs idle for 7+ days are closed automatically.

## Using the paymaster

Endpoint: `https://deployer.devnet.solana.com`

```bash
cargo install kora-deploy
kora-deploy --program-so ./my-program.so
```

That's it. The CLI handles `getPayerSigner` → buffer creation → chunked write →
deploy, all signed by the paymaster.

Constraints:
- The paymaster is fee payer AND upgrade authority on every loader instruction.
  Transactions with any other authority are rejected.
- Programs are not yours to close — authority belongs to the paymaster. Idle
  programs (no on-chain activity in 7 days) are reaped automatically; the rent
  goes back to the paymaster.

`kora-deploy` source lives in [`crates/kora-deploy/`](../../crates/kora-deploy);
the canonical published artifact is on [crates.io](https://crates.io/crates/kora-deploy).

## Deploying your own paymaster

Runs on GCP Cloud Run, signs via GCP KMS (Ed25519), uses Redis for usage limits,
and ships a daily Cloud Run Job that closes idle programs.

### Files

- `Dockerfile` — installs `kora-cli` from `main` and runs `kora rpc start`.
- `Dockerfile.reaper` + `cloudbuild.reaper.yaml` — builds the
  `devnet_deploy_reaper` binary that closes idle programs daily.
- `kora.toml` — allowlists loader-v3 + loader-v4, enables the
  `DeployAuthority` plugin, sets the fee-payer policy.
- `signers.toml` — single GCP KMS signer; key name + pubkey come from env.
- `src/reaper/` + `src/bin/reaper.rs` — reaper source.
- `src/bin/devnet_smoke.rs` — CI smoke test that exercises the full
  deploy → verify-authority → close lifecycle against a live paymaster.

### One-time GCP setup

The following resources must exist in the target GCP project before the
workflow can run. Re-running the workflow does not re-create them.

1. **KMS** — Ed25519 key in a keyring (e.g. `projects/$PROJECT/locations/global/keyRings/kora-devnet/cryptoKeys/paymaster`).
2. **Memorystore Redis** — a basic-tier instance reachable from the Cloud Run
   service via VPC connector.
3. **VPC Serverless Connector** — bridges Cloud Run to the Memorystore VPC.
4. **Cloud Run service** — empty service in the target project/region; the
   workflow updates revisions, it does not create the service.
5. **Workload Identity Federation pool + provider** — trusts
   `solana-foundation/kora` (filter on `attribute.repository`) and is bound to
   a deployer service account holding **only** these scoped roles:
   - `roles/run.admin` scoped to the one Cloud Run service
   - `roles/iam.serviceAccountUser` scoped to the runtime service account
   - `roles/cloudbuild.builds.editor` (project — required by `gcloud builds submit`)
   - `roles/artifactregistry.writer` scoped to the `cloud-run-source-deploy` AR repo
   - `roles/storage.objectAdmin` scoped to `gs://<PROJECT>_cloudbuild` (build staging only)
   - The runtime SA holds `roles/cloudkms.signer` on the KMS key. The deployer
     SA does not need KMS access — it only orchestrates deploys.
6. **Cloud Run Job** for the reaper; runtime SA needs `roles/cloudkms.signer`
   on the KMS key. The workflow updates it; doesn't create it.
7. **Cloud Scheduler** entry triggering the reaper Job's `:run` endpoint
   (suggested `0 3 * * *` UTC) with its own SA as invoker.

### Environment variables

The deploy workflow expects these to be available in the job env before any
`gcloud` step runs. Source them from whatever secret store you use.

| Variable | Example | Notes |
| --- | --- | --- |
| `GCP_PROJECT_ID` | `solana-kora-devnet` | |
| `GCP_REGION` | `us-central1` | |
| `GCP_WIF_PROVIDER` | `projects/123/locations/global/workloadIdentityPools/github/providers/kora` | |
| `GCP_DEPLOYER_SERVICE_ACCOUNT` | `kora-deployer@solana-kora-devnet.iam.gserviceaccount.com` | |
| `CLOUD_RUN_SERVICE` | `kora-devnet-paymaster` | |
| `CLOUD_RUN_REAPER_JOB` | `kora-devnet-reaper` | Cloud Run Job for the reaper. |
| `VPC_CONNECTOR` | `kora-vpc-connector` | |
| `DEVNET_RPC_URL` | `https://api.devnet.solana.com` | mapped to `RPC_URL` on the Cloud Run revision |
| `DEVNET_KORA_GCP_KMS_KEY_NAME` | `projects/.../cryptoKeys/paymaster/cryptoKeyVersions/1` | mapped to `KORA_GCP_KMS_KEY_NAME` |
| `DEVNET_KORA_GCP_KMS_PUBLIC_KEY` | base58 pubkey derived from the KMS public key | mapped to `KORA_GCP_KMS_PUBLIC_KEY` |
| `DEVNET_KORA_REDIS_URL` | `redis://10.x.y.z:6379` | private VPC IP, mapped to `KORA_REDIS_URL` |

The 4 runtime vars carry a `DEVNET_` prefix so they don't collide with names
the kora binary and the integration test runner read directly (`RPC_URL`,
`KORA_REDIS_URL`, `KORA_GCP_KMS_KEY_NAME`, `KORA_GCP_KMS_PUBLIC_KEY`).

### Triggering a deploy

GitHub → **Actions** → **Deploy devnet paymaster** → **Run workflow**. Pick
`deploy_target` (`both` / `rpc` / `reaper`, default `both`) and optional git
ref.

### Reaper

`devnet_deploy_reaper` runs as a Cloud Run Job: discover programs via
`getProgramAccounts` filtered on upgrade authority → classify via
`getSignaturesForAddress(limit=1)` (slot fallback) → close. v3 uses
`close_any`; v4 uses `Retract` + `SetProgramLength(0)`. Audit trail =
Cloud Logging + on-chain signatures.

Manual trigger:

```bash
gcloud run jobs execute "$CLOUD_RUN_REAPER_JOB" \
    --region "$GCP_REGION" --project "$GCP_PROJECT_ID"
```

Local dry-run:

```bash
cargo run --release --bin devnet_deploy_reaper -- \
    --config examples/devnet-deploy-paymaster/kora.toml \
    --signers-config examples/devnet-deploy-paymaster/signers.toml \
    --threshold-hours 168 --dry-run
```

Flags: `--threshold-hours`, `--dry-run`, `--max-closes`.
