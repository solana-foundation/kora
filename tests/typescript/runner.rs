use crate::common::{harness::started, KoraSpec};
use std::{path::PathBuf, process::Stdio};
use tokio::process::Command;

/// Runs the TypeScript integration suite against a Kora node on the embedded
/// surfnet. The suite seeds nothing: it reuses the accounts the harness wrote,
/// including the flavor wallets in `seed.rs`.
pub async fn run_suite(spec: KoraSpec, flavor_env: &[(&str, &str)]) {
    let harness = started(spec).await;

    let sdk_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("tests/ always has a parent")
        .join("sdks/ts");

    let status = Command::new("pnpm")
        .current_dir(&sdk_dir)
        .args(["exec", "jest", "test/integration.test.ts", "--runInBand"])
        .env("KORA_RPC_URL", &harness.server_url)
        .env("SOLANA_RPC_URL", &harness.rpc_url)
        .env("SOLANA_WS_URL", &harness.ws_url)
        .envs(flavor_env.iter().copied())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()
        .await
        .expect("failed to spawn pnpm; run 'pnpm install' in sdks/ts first");

    assert!(status.success(), "typescript integration suite failed: {status}");
}
