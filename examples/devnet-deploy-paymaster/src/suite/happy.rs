use std::path::Path;

use anyhow::Result;
use kora_deploy::{close, deploy, verify_upgrade_authority, DeployConfig};
use solana_sdk::pubkey::Pubkey;

pub async fn run(kora_url: &str, rpc_url: &str, program_so: &Path) -> Result<()> {
    let user_id = format!("kora-smoke-{}", Pubkey::new_unique());
    let result =
        deploy(&DeployConfig { kora_url, rpc_url, program_so, user_id: user_id.clone() }).await?;
    println!("  deployed program {}", result.program);

    verify_upgrade_authority(rpc_url, &result.program_data, &result.kora_pubkey).await?;
    println!("  verified upgrade_authority == {}", result.kora_pubkey);

    let sig = close(
        rpc_url,
        kora_url,
        &user_id,
        &result.kora_pubkey,
        &result.program,
        &result.program_data,
    )
    .await?;
    println!("  closed program (sig {sig})");
    Ok(())
}
