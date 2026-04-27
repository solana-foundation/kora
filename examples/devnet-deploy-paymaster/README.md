# Devnet Deploy Paymaster

Reference Kora paymaster that sponsors program deploys on Solana devnet, with
loader-v3 / loader-v4 upgrade authority pinned to Kora so users can't drain
program-data rent. Deploys to GCP Cloud Run, signs via GCP KMS (Ed25519), and
uses Memorystore Redis for usage limits.

## Files in this directory

- `Dockerfile` — installs `kora-cli` from the `main` branch (until a release
  ships with the `DeployAuthority` plugin + `KORA_REDIS_URL` support) and runs
  `kora rpc start` on port `$PORT`.
- `kora.toml` — paymaster config: allowlists loader-v3 + loader-v4, enables the
  `DeployAuthority` plugin, sets the loader-v3/v4 fee-payer policy.
- `signers.toml` — single GCP KMS signer; key name + public key come from env.

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
   a deployer service account that holds:
   - `roles/run.admin` on the Cloud Run service
   - `roles/iam.serviceAccountUser` on the runtime service account
   - `roles/cloudkms.signer` on the KMS key (the runtime SA needs this; the
     deployer SA only needs it if it ever invokes KMS directly)

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
| `DEVNET_RPC_URL` | `https://api.devnet.solana.com` | mapped to `RPC_URL` on the Cloud Run revision; prefixed to avoid colliding with the integration test runner that reads `$RPC_URL` |
| `KORA_GCP_KMS_KEY_NAME` | `projects/.../cryptoKeys/paymaster/cryptoKeyVersions/1` | |
| `KORA_GCP_KMS_PUBLIC_KEY` | base58 Solana pubkey derived from the KMS public key | |
| `KORA_REDIS_URL` | `redis://10.x.y.z:6379` | private VPC IP |

`KORA_GCP_KMS_PUBLIC_KEY` is derived once from the PEM that
`gcloud kms keys versions get-public-key` returns; it never changes for a
given key version. `KORA_REDIS_URL` is the private VPC IP — also stable.

The Doppler service identity (`DOPPLER_SERVICE_IDENTITY_ID`) and project
(`DOPPLER_PROJECT`, defaults to `kora`) live in repo Variables — already
configured for the rest of the CI workflows.

### Triggering a deploy

GitHub → **Actions** → **Deploy devnet paymaster** → **Run workflow**.
Optionally specify a git ref (defaults to `main`).
