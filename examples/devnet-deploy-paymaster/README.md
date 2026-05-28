# Devnet Deploy Paymaster

Reference Kora paymaster that sponsors program deploys on Solana devnet, with
loader-v3 / loader-v4 upgrade authority pinned to Kora so users can't drain
program-data rent. Deploys to GCP Cloud Run, signs via GCP KMS (Ed25519), and
uses Memorystore Redis for usage limits.

## Files in this directory

- `Dockerfile` — installs `kora-cli` from the `main` branch (until a release
  ships with the `DeployAuthority` plugin + `KORA_REDIS_URL` support) and runs
  `kora rpc start` on port `$PORT`.
- `Dockerfile.reaper` — multi-stage build of the `devnet_deploy_reaper` binary
  from the local workspace source. Shipped as a separate image because the
  reaper isn't published on crates.io and needs path deps on `kora-lib`.
- `cloudbuild.reaper.yaml` — Cloud Build config that points at
  `Dockerfile.reaper` with the workspace root as build context.
- `kora.toml` — paymaster config: allowlists loader-v3 + loader-v4, enables the
  `DeployAuthority` plugin, sets the loader-v3/v4 fee-payer policy.
- `signers.toml` — single GCP KMS signer; key name + public key come from env.
- `src/reaper/` + `src/bin/reaper.rs` — Rust source for the reaper binary that
  closes paymaster-owned programs after a configurable idle threshold.

## Deploying

Triggered manually via the **Deploy devnet paymaster** workflow in the GitHub
Actions tab. The workflow uses Workload Identity Federation — no service
account JSON keys live in the repo.

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
   a deployer service account holding **only** these scoped roles (no
   project-wide storage perms — important when the GCP project is shared):
   - `roles/run.admin` scoped to the one Cloud Run service
   - `roles/iam.serviceAccountUser` scoped to the runtime service account
   - `roles/cloudbuild.builds.editor` (project — required by `gcloud builds submit`)
   - `roles/artifactregistry.writer` scoped to the `cloud-run-source-deploy` AR repo
   - `roles/storage.objectAdmin` scoped to `gs://<PROJECT>_cloudbuild` (build staging only)
   - The runtime SA holds `roles/cloudkms.signer` on the KMS key. The deployer
     SA does not need KMS access — it only orchestrates deploys.

### Doppler config

Deploy values live in the same `stg_github` Doppler config as the rest of
the CI workflows. The workflow fetches them via `dopplerhq/secrets-fetch-action`
with OIDC and injects them as env vars in subsequent steps.

| Doppler key | Example | Notes |
| --- | --- | --- |
| `GCP_PROJECT_ID` | `solana-kora-devnet` | |
| `GCP_REGION` | `us-central1` | |
| `GCP_WIF_PROVIDER` | `projects/123/locations/global/workloadIdentityPools/github/providers/kora` | |
| `GCP_DEPLOYER_SERVICE_ACCOUNT` | `kora-deployer@solana-kora-devnet.iam.gserviceaccount.com` | |
| `CLOUD_RUN_SERVICE` | `kora-devnet-paymaster` | |
| `VPC_CONNECTOR` | `kora-vpc-connector` | |
| `DEVNET_RPC_URL` | `https://api.devnet.solana.com` | mapped to `RPC_URL` on the Cloud Run revision |
| `DEVNET_KORA_GCP_KMS_KEY_NAME` | `projects/.../cryptoKeys/paymaster/cryptoKeyVersions/1` | mapped to `KORA_GCP_KMS_KEY_NAME` |
| `DEVNET_KORA_GCP_KMS_PUBLIC_KEY` | base58 Solana pubkey derived from the KMS public key | mapped to `KORA_GCP_KMS_PUBLIC_KEY` |
| `DEVNET_KORA_REDIS_URL` | `redis://10.x.y.z:6379` | private VPC IP, mapped to `KORA_REDIS_URL` |

The 4 runtime keys carry a `DEVNET_` prefix so they don't collide with what
the kora binary and the integration test runner read directly — `RPC_URL`,
`KORA_REDIS_URL`, `KORA_GCP_KMS_KEY_NAME`, `KORA_GCP_KMS_PUBLIC_KEY` would
otherwise leak into every CI job that loads `stg_github`.

The Doppler service identity (`DOPPLER_SERVICE_IDENTITY_ID`) and project
(`DOPPLER_PROJECT`, defaults to `kora`) live in repo Variables — already
configured for the rest of the CI workflows.

### Triggering a deploy

GitHub → **Actions** → **Deploy devnet paymaster** → **Run workflow**.
Optionally specify a git ref (defaults to `main`).

## Reaper (inactive-program cleanup)

The paymaster keeps upgrade authority on every program it sponsors so users
can't close the program and drain the rent SOL. The `devnet_deploy_reaper`
binary in this crate runs on a daily Cloud Run Job, scans for programs that
haven't seen on-chain activity in 7 days, and closes them — recovering the
rent back to the fee payer.

### How it works

1. **Discovery**: scans the chain for programs whose upgrade authority is the
   paymaster's fee payer. Two `getProgramAccounts` queries for loader-v3
   (`ProgramData` accounts filtered by authority + an index of all `Program`
   pointer accounts) and one for loader-v4 (state account filtered by
   authority offset).
2. **Activity**: per program, `getSignaturesForAddress(limit=1)` and compare
   the newest `block_time` to `now − threshold`. Fallback to
   `getBlockTime(last_state_slot)` when no signatures are returned.
3. **Close**: for v3, a single `close_any(program_data, fee_payer, fee_payer,
   program)` instruction; for v4, `Retract` + `SetProgramLength(0,
   recipient=fee_payer)` in one transaction. Signed locally via the same
   `SignerPool` the RPC uses.

The audit trail is whatever lands in Cloud Logging plus the on-chain close
signatures — there is no extra DB or Redis record.

### One-time GCP setup (in addition to the RPC setup above)

7. **Cloud Run Job** named to match the `CLOUD_RUN_REAPER_JOB` Doppler key
   (e.g. `kora-devnet-reaper`). Created once with the same KMS-signing service
   account the RPC uses. The deploy workflow runs `gcloud run jobs update`
   against it — it does not create the Job.
8. **Cloud Scheduler entry** triggering the Job on a cron (recommend
   `0 3 * * *` UTC). Targets the Cloud Run Job's `:run` endpoint with the
   scheduler's own service account as the invoker.
9. **Reaper runtime service account permissions**: the runtime SA on the Job
   needs `roles/cloudkms.signer` on the KMS key (same as the RPC's runtime
   SA). It does **not** need Redis access — the reaper doesn't use Redis.

### Doppler key to add

| Doppler key | Example | Notes |
| --- | --- | --- |
| `CLOUD_RUN_REAPER_JOB` | `kora-devnet-reaper` | Name of the Cloud Run Job created in step 7 above. |

### Manual trigger

```bash
# Production: trigger the Cloud Run Job out-of-band.
gcloud run jobs execute "$CLOUD_RUN_REAPER_JOB" --region "$GCP_REGION" --project "$GCP_PROJECT_ID"

# Local dry-run: log what would close, change nothing on-chain.
KORA_GCP_KMS_KEY_NAME=...  KORA_GCP_KMS_PUBLIC_KEY=... \
  cargo run --release --bin devnet_deploy_reaper -- \
    --config examples/devnet-deploy-paymaster/kora.toml \
    --signers-config examples/devnet-deploy-paymaster/signers.toml \
    --rpc-url https://api.devnet.solana.com \
    --threshold 7d \
    --dry-run
```

Flags: `--threshold` (humantime: `7d`, `48h`), `--dry-run`,
`--max-closes <n>` (cap per invocation), `--loader v3|v4|both`.
