# Devnet Deploy Paymaster

Reference Kora paymaster that sponsors program deploys on Solana devnet, with
loader-v3 / loader-v4 upgrade authority pinned to Kora so users can't drain
program-data rent. Deploys to GCP Cloud Run, signs via GCP KMS (Ed25519), and
uses Memorystore Redis for usage limits.

## Files in this directory

- `Dockerfile` — installs `kora-cli` from the `main` branch (until a release
  ships with the `DeployAuthority` plugin + `KORA_REDIS_URL` support) and runs
  `kora rpc start` on port `$PORT`.
- `Dockerfile.reaper` + `cloudbuild.reaper.yaml` — build for the
  `devnet_deploy_reaper` binary (workspace-context, multi-stage).
- `kora.toml` — paymaster config: allowlists loader-v3 + loader-v4, enables the
  `DeployAuthority` plugin, sets the loader-v3/v4 fee-payer policy.
- `signers.toml` — single GCP KMS signer; key name + public key come from env.
- `src/reaper/` + `src/bin/reaper.rs` — reaper source (see Reaper section).

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
6. **Cloud Run Job** for the reaper (see Reaper section below); runtime SA
   needs `roles/cloudkms.signer` on the KMS key. Workflow updates it; doesn't
   create it.
7. **Cloud Scheduler** entry triggering the reaper Job's `:run` endpoint
   (suggested `0 3 * * *` UTC) with its own SA as invoker.

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
| `CLOUD_RUN_REAPER_JOB` | `kora-devnet-reaper` | Cloud Run Job for the reaper. |
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

## Reaper

`devnet_deploy_reaper` runs daily as a Cloud Run Job, finds paymaster-owned
programs idle past the threshold (default 7d), and closes them — rent goes
back to the fee payer.

Flow: discover via `getProgramAccounts` filtered on upgrade authority →
classify via `getSignaturesForAddress(limit=1)` (slot fallback) → close.
v3 uses `close_any`; v4 uses `Retract` + `SetProgramLength(0)`. Audit trail
= Cloud Logging + on-chain signatures.

### Manual trigger

```bash
# Trigger the Job out-of-band.
gcloud run jobs execute "$CLOUD_RUN_REAPER_JOB" --region "$GCP_REGION" --project "$GCP_PROJECT_ID"

# Local dry-run.
cargo run --release --bin devnet_deploy_reaper -- \
    --config examples/devnet-deploy-paymaster/kora.toml \
    --signers-config examples/devnet-deploy-paymaster/signers.toml \
    --threshold 7d --dry-run
```

Flags: `--threshold`, `--dry-run`, `--max-closes`, `--loader v3|v4|both`.
