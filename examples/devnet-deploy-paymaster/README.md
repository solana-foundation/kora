# Devnet Deploy Paymaster

A Kora paymaster that sponsors program deploys on Solana devnet so devs don't
have to drip-faucet several SOL. Upgrade authority is pinned to the paymaster
so the SOL can't be drained; programs idle for 7+ days are closed automatically.

## Using the paymaster

Endpoint: `https://deployer.devnet.solana.com`

Constraints:
- The paymaster must be **fee payer AND upgrade authority** on every loader
  instruction. Transactions with any other authority are rejected.
- Programs are not yours to close — authority belongs to the paymaster. Idle
  programs (no on-chain activity in 7 days) are reaped automatically; the rent
  goes back to the paymaster.

Get the paymaster's pubkey:

```bash
curl -sS https://deployer.devnet.solana.com -H 'content-type: application/json' \
  -d '{"jsonrpc":"2.0","id":1,"method":"getPayerSigner","params":[]}' \
  | jq -r .result.signer_address
```

Deploy flow (Rust, using `solana-loader-v3-interface`):

```rust
use solana_loader_v3_interface::{instruction as loader_v3, state::UpgradeableLoaderState};
use solana_sdk::{message::Message, signature::Keypair, signer::Signer, transaction::Transaction};

let paymaster: Pubkey = /* fetched via getPayerSigner */;
let buffer = Keypair::new();
let program = Keypair::new();
let elf: Vec<u8> = std::fs::read("program.so")?;

// 1. create_buffer (paymaster is buffer authority + funder)
let buffer_lamports = rpc
    .get_minimum_balance_for_rent_exemption(UpgradeableLoaderState::size_of_buffer(elf.len()))
    .await?;
let ixs = loader_v3::create_buffer(&paymaster, &buffer.pubkey(), &paymaster, buffer_lamports, elf.len())?;
submit(&rpc, &paymaster, &ixs, &[&buffer]).await?;

// 2. write the .so in ≤900-byte chunks
for (i, chunk) in elf.chunks(900).enumerate() {
    let ix = loader_v3::write(&buffer.pubkey(), &paymaster, (i * 900) as u32, chunk.to_vec());
    submit(&rpc, &paymaster, &[ix], &[]).await?;
}

// 3. deploy (paymaster becomes upgrade authority + funder)
let program_lamports = rpc
    .get_minimum_balance_for_rent_exemption(UpgradeableLoaderState::size_of_program())
    .await?;
let ixs = loader_v3::deploy_with_max_program_len(
    &paymaster, &program.pubkey(), &buffer.pubkey(), &paymaster,
    program_lamports, elf.len(),
)?;
submit(&rpc, &paymaster, &ixs, &[&program]).await?;
```

Where `submit` builds the message with `fee_payer = paymaster`, partially
signs with the extra keypairs, base64-encodes, and POSTs:

```rust
async fn submit(rpc: &RpcClient, paymaster: &Pubkey, ixs: &[Instruction], extra_signers: &[&Keypair]) -> Result<Signature> {
    let blockhash = rpc.get_latest_blockhash().await?;
    let mut tx = Transaction::new_unsigned(Message::new_with_blockhash(ixs, Some(paymaster), &blockhash));
    if !extra_signers.is_empty() {
        tx.partial_sign(extra_signers, blockhash);
    }
    let tx_b64 = base64::encode(bincode::serialize(&tx)?);
    let resp: Value = http
        .post("https://deployer.devnet.solana.com")
        .json(&json!({
            "jsonrpc": "2.0", "id": 1,
            "method": "signAndSendTransaction",
            "params": { "transaction": tx_b64, "user_id": user_id }
        }))
        .send().await?.json().await?;
    Signature::from_str(resp["result"]["signature"].as_str().unwrap())
}
```

`user_id` is an arbitrary string the paymaster buckets by for usage limits.

## Deploying your own paymaster

Runs on GCP Cloud Run, signs via GCP KMS (Ed25519), uses Redis for usage
limits, and ships a daily Cloud Run Job that closes idle programs.

### Files

- `Dockerfile` — installs `kora-cli` from `main` and runs `kora rpc start`.
- `Dockerfile.reaper` + `cloudbuild.reaper.yaml` — builds the
  `devnet_deploy_reaper` binary that closes idle programs daily.
- `kora.toml` — allowlists loader-v3 + loader-v4, enables the
  `DeployAuthority` plugin, sets the fee-payer policy.
- `signers.toml` — single GCP KMS signer; key name + pubkey come from env.
- `src/reaper/` + `src/bin/reaper.rs` — reaper source.

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
