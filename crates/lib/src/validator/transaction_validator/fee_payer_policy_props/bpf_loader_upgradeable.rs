use super::{assert_role_gated_iff_flag_off, role_and_flags, DrainRole};
use crate::{config::FeePayerPolicy, constant::BPF_LOADER_UPGRADEABLE_PROGRAM_ID};
use proptest::prelude::*;
use solana_loader_v3_interface::instruction::{
    close_any, create_buffer, deploy_with_max_program_len, extend_program, extend_program_checked,
    migrate_program, set_upgrade_authority, set_upgrade_authority_checked, upgrade, write,
};
use solana_sdk::{instruction::Instruction, pubkey::Pubkey};

#[derive(Debug, Clone, Copy)]
enum BpfLoaderUpgradeableRole {
    InitializeBuffer,
    Write,
    DeployWithMaxDataLenPayer,
    DeployWithMaxDataLenUpgradeAuthority,
    Upgrade,
    SetAuthorityCurrent,
    SetAuthorityNew,
    SetAuthorityCheckedCurrent,
    SetAuthorityCheckedNew,
    Close,
    ExtendProgram,
    ExtendProgramCheckedAuthority,
    ExtendProgramCheckedPayer,
    Migrate,
}

impl DrainRole for BpfLoaderUpgradeableRole {
    const ROLES: &'static [Self] = &[
        Self::InitializeBuffer,
        Self::Write,
        Self::DeployWithMaxDataLenPayer,
        Self::DeployWithMaxDataLenUpgradeAuthority,
        Self::Upgrade,
        Self::SetAuthorityCurrent,
        Self::SetAuthorityNew,
        Self::SetAuthorityCheckedCurrent,
        Self::SetAuthorityCheckedNew,
        Self::Close,
        Self::ExtendProgram,
        Self::ExtendProgramCheckedAuthority,
        Self::ExtendProgramCheckedPayer,
        Self::Migrate,
    ];

    fn allowed_programs() -> Vec<String> {
        vec![BPF_LOADER_UPGRADEABLE_PROGRAM_ID.to_string()]
    }

    fn flag(self, policy: &mut FeePayerPolicy) -> &mut bool {
        match self {
            Self::InitializeBuffer => &mut policy.bpf_loader_upgradeable.allow_initialize_buffer,
            Self::Write => &mut policy.bpf_loader_upgradeable.allow_write,
            Self::DeployWithMaxDataLenPayer | Self::DeployWithMaxDataLenUpgradeAuthority => {
                &mut policy.bpf_loader_upgradeable.allow_deploy_with_max_data_len
            }
            Self::Upgrade => &mut policy.bpf_loader_upgradeable.allow_upgrade,
            Self::SetAuthorityCurrent | Self::SetAuthorityNew => {
                &mut policy.bpf_loader_upgradeable.allow_set_authority
            }
            Self::SetAuthorityCheckedCurrent | Self::SetAuthorityCheckedNew => {
                &mut policy.bpf_loader_upgradeable.allow_set_authority_checked
            }
            Self::Close => &mut policy.bpf_loader_upgradeable.allow_close,
            Self::ExtendProgram => &mut policy.bpf_loader_upgradeable.allow_extend_program,
            Self::ExtendProgramCheckedAuthority | Self::ExtendProgramCheckedPayer => {
                &mut policy.bpf_loader_upgradeable.allow_extend_program_checked
            }
            Self::Migrate => &mut policy.bpf_loader_upgradeable.allow_migrate,
        }
    }

    fn instruction(self, actor: &Pubkey) -> Instruction {
        let program = Pubkey::new_unique();
        let buffer = Pubkey::new_unique();
        let other = Pubkey::new_unique();
        match self {
            // create_buffer and deploy_with_max_program_len each return the funding
            // create_account alongside the loader instruction; only the latter is under test.
            Self::InitializeBuffer => {
                create_buffer(&other, &buffer, actor, 0, 0).unwrap().swap_remove(1)
            }
            Self::Write => write(&buffer, actor, 0, vec![1, 2, 3]),
            Self::DeployWithMaxDataLenPayer => {
                deploy_with_max_program_len(actor, &program, &buffer, &other, 0, 0)
                    .unwrap()
                    .swap_remove(1)
            }
            Self::DeployWithMaxDataLenUpgradeAuthority => {
                deploy_with_max_program_len(&other, &program, &buffer, actor, 0, 0)
                    .unwrap()
                    .swap_remove(1)
            }
            // `spill` is a lamport sink and deliberately ungated, so the actor only ever
            // occupies the upgrade authority slot here.
            Self::Upgrade => upgrade(&program, &buffer, actor, &other),
            Self::SetAuthorityCurrent => set_upgrade_authority(&program, actor, Some(&other)),
            Self::SetAuthorityNew => set_upgrade_authority(&program, &other, Some(actor)),
            Self::SetAuthorityCheckedCurrent => {
                set_upgrade_authority_checked(&program, actor, &other)
            }
            Self::SetAuthorityCheckedNew => set_upgrade_authority_checked(&program, &other, actor),
            // The drainage guard rejects a fee-payer authority paired with a foreign recipient
            // regardless of the flag, so this role names the actor in both slots to isolate
            // the flag as the only thing under test.
            Self::Close => close_any(&buffer, actor, Some(actor), None),
            Self::ExtendProgram => extend_program(&program, Some(actor), 1),
            Self::ExtendProgramCheckedAuthority => {
                extend_program_checked(&program, actor, Some(&other), 1)
            }
            Self::ExtendProgramCheckedPayer => {
                extend_program_checked(&program, &other, Some(actor), 1)
            }
            Self::Migrate => migrate_program(&buffer, &program, actor),
        }
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(512))]

    #[test]
    fn fee_payer_role_gated_iff_flag_off(
        (role_idx, actor_is_fee_payer, flags) in role_and_flags::<BpfLoaderUpgradeableRole>(),
    ) {
        assert_role_gated_iff_flag_off::<BpfLoaderUpgradeableRole>(role_idx, actor_is_fee_payer, &flags)?;
    }
}
