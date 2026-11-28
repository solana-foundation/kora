use std::{env, fs, path::Path};

use anyhow::{bail, Result};
use kora_deploy::{
    close, deploy, upgrade, verify_upgrade_authority, DeployConfig, UpgradeConfig, UpgradeOutcome,
};
use solana_sdk::{signature::Keypair, signer::Signer};

pub async fn run(kora_url: &str, rpc_url: &str, program_so: &Path) -> Result<()> {
    let user_id = format!("kora-smoke-{}", Keypair::new().pubkey());
    let owner = Keypair::new();

    let result = deploy(&DeployConfig {
        kora_url,
        rpc_url,
        program_so,
        user_id: user_id.clone(),
        wallet: Some(&owner),
        resume: false,
        cleanup_on_failure: true,
        state_path: env::temp_dir()
            .join(format!(".kora-deploy-state-{}.json", Keypair::new().pubkey())),
    })
    .await?;
    println!("  deployed program {} (owner {})", result.program, owner.pubkey());

    verify_upgrade_authority(rpc_url, &result.program_data, &result.kora_pubkey).await?;
    println!("  verified upgrade_authority == {}", result.kora_pubkey);

    // Mutate binary to force an on-chain upgrade transaction.
    let mut upgrade_bytes = fs::read(program_so)?;
    mutate_binary_for_upgrade(&mut upgrade_bytes);
    let upgrade_so =
        env::temp_dir().join(format!(".kora-smoke-upgrade-{}.so", Keypair::new().pubkey()));
    fs::write(&upgrade_so, &upgrade_bytes)?;

    let outcome = upgrade(&UpgradeConfig {
        kora_url,
        rpc_url,
        program_so: &upgrade_so,
        user_id: user_id.clone(),
        program: result.program,
        wallet: &owner,
        resume: false,
        cleanup_on_failure: true,
        state_path: env::temp_dir()
            .join(format!(".kora-upgrade-state-{}.json", Keypair::new().pubkey())),
    })
    .await;
    let _ = fs::remove_file(&upgrade_so);
    let outcome = outcome?;
    match outcome {
        UpgradeOutcome::Submitted(s) => {
            println!("  upgraded program as registered owner (sig {s})");
        }
        UpgradeOutcome::AlreadyUpToDate => {
            bail!(
                "expected live upgrade transaction to be submitted, but binary was reported as already up to date"
            );
        }
        UpgradeOutcome::AlreadyLanded => {
            bail!(
                "expected live upgrade transaction to be submitted, but upgrade was reported as already landed"
            );
        }
    }

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

// Mutates a byte in .rodata to produce a new binary hash without changing file
// length, since ProgramData accounts cannot expand during upgrade on-chain.
fn mutate_binary_for_upgrade(bytes: &mut [u8]) {
    if let Some(byte) = find_rodata_offset(bytes).and_then(|offset| bytes.get_mut(offset)) {
        *byte ^= 0xFF;
    } else if let Some(last) = bytes.last_mut() {
        *last ^= 0xFF;
    }
}

// Scans section headers of a 64-bit ELF to locate .rodata without external parser crates.
fn find_rodata_offset(data: &[u8]) -> Option<usize> {
    if data.len() < 64 || &data[..4] != b"\x7fELF" || data[4] != 2 || data[5] != 1 {
        return None;
    }
    let e_shoff = u64::from_le_bytes(data[40..48].try_into().ok()?) as usize;
    let e_shentsize = u16::from_le_bytes(data[58..60].try_into().ok()?) as usize;
    let e_shnum = u16::from_le_bytes(data[60..62].try_into().ok()?) as usize;
    let e_shstrndx = u16::from_le_bytes(data[62..64].try_into().ok()?) as usize;

    if e_shentsize < 64 || e_shstrndx >= e_shnum {
        return None;
    }

    let strtab_hdr = e_shoff.checked_add(e_shstrndx.checked_mul(e_shentsize)?)?;
    let strtab_offset =
        u64::from_le_bytes(data.get(strtab_hdr + 24..strtab_hdr + 32)?.try_into().ok()?) as usize;
    let strtab_size =
        u64::from_le_bytes(data.get(strtab_hdr + 32..strtab_hdr + 40)?.try_into().ok()?) as usize;
    let shstrtab = data.get(strtab_offset..strtab_offset.checked_add(strtab_size)?)?;

    for i in 0..e_shnum {
        let Some(hdr) = e_shoff.checked_add(i.checked_mul(e_shentsize)?) else {
            continue;
        };
        let Some(hdr_bytes) = data.get(hdr..hdr.checked_add(e_shentsize)?) else {
            continue;
        };
        let name_idx = u32::from_le_bytes(hdr_bytes.get(..4)?.try_into().ok()?) as usize;
        let sec_offset = u64::from_le_bytes(hdr_bytes.get(24..32)?.try_into().ok()?) as usize;
        let sec_size = u64::from_le_bytes(hdr_bytes.get(32..40)?.try_into().ok()?) as usize;

        if sec_size > 0
            && sec_offset < data.len()
            && shstrtab.get(name_idx..).is_some_and(|name| name.starts_with(b".rodata\0"))
        {
            return Some(sec_offset);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mutate_binary_for_upgrade_preserves_length_and_modifies_data() {
        let fixture_path = Path::new(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../tests/src/common/transfer-hook-example/transfer_hook_example.so"
        ));
        let mut bytes = fs::read(fixture_path).expect("failed to read fixture");
        let original = bytes.clone();

        mutate_binary_for_upgrade(&mut bytes);

        assert_eq!(bytes.len(), original.len(), "length must remain identical");
        assert_ne!(bytes, original, "data must differ");
    }

    #[test]
    fn test_mutate_binary_for_upgrade_fallback_without_rodata() {
        let mut bytes = vec![1, 2, 3, 4];
        mutate_binary_for_upgrade(&mut bytes);
        assert_eq!(bytes, vec![1, 2, 3, 4 ^ 0xFF]);
    }

    #[test]
    fn test_mutate_binary_for_upgrade_empty_does_not_panic() {
        let mut bytes = vec![];
        mutate_binary_for_upgrade(&mut bytes);
        assert!(bytes.is_empty());
    }
}
