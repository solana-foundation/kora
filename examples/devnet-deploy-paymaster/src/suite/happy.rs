use std::path::Path;

use anyhow::Result;
use kora_deploy::{close, deploy, upgrade, verify_upgrade_authority, DeployConfig, UpgradeConfig};
use solana_sdk::{pubkey::Pubkey, signature::Keypair, signer::Signer};

pub async fn run(kora_url: &str, rpc_url: &str, program_so: &Path) -> Result<()> {
    let user_id = format!("kora-smoke-{}", Pubkey::new_unique());
    let owner = Keypair::new();

    let result = deploy(&DeployConfig {
        kora_url,
        rpc_url,
        program_so,
        user_id: user_id.clone(),
        wallet: Some(&owner),
    })
    .await?;
    println!("  deployed program {} (owner {})", result.program, owner.pubkey());

    verify_upgrade_authority(rpc_url, &result.program_data, &result.kora_pubkey).await?;
    println!("  verified upgrade_authority == {}", result.kora_pubkey);

    let sig = upgrade(&UpgradeConfig {
        kora_url,
        rpc_url,
        program_so,
        user_id: user_id.clone(),
        program: result.program,
        wallet: &owner,
    })
    .await?;
    println!("  upgraded program as registered owner (sig {sig})");

    let sig = close(
        rpc_url,
        kora_url,
        &user_id,
        &result.kora_pubkey,
        &result.program,
        &result.program_data,
        Some(&owner),
    )
    .await?;
    println!("  closed program as registered owner (sig {sig})");
    Ok(())
}
