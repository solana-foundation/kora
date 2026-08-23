use super::{assert_role_gated_iff_flag_off, role_and_flags, DrainRole};
use crate::{config::FeePayerPolicy, constant::LOADER_V4_PROGRAM_ID};
use proptest::prelude::*;
use solana_loader_v4_interface::instruction::{
    copy, deploy, finalize, retract, set_program_length, transfer_authority, write,
};
use solana_sdk::{instruction::Instruction, pubkey::Pubkey};

#[derive(Debug, Clone, Copy)]
enum LoaderV4Role {
    Write,
    Copy,
    SetProgramLengthAuthority,
    SetProgramLengthRecipient,
    Deploy,
    Retract,
    TransferAuthorityCurrent,
    TransferAuthorityNew,
    Finalize,
}

impl DrainRole for LoaderV4Role {
    const ROLES: &'static [Self] = &[
        Self::Write,
        Self::Copy,
        Self::SetProgramLengthAuthority,
        Self::SetProgramLengthRecipient,
        Self::Deploy,
        Self::Retract,
        Self::TransferAuthorityCurrent,
        Self::TransferAuthorityNew,
        Self::Finalize,
    ];

    fn allowed_programs() -> Vec<String> {
        vec![LOADER_V4_PROGRAM_ID.to_string()]
    }

    fn flag(self, policy: &mut FeePayerPolicy) -> &mut bool {
        match self {
            Self::Write => &mut policy.loader_v4.allow_write,
            Self::Copy => &mut policy.loader_v4.allow_copy,
            Self::SetProgramLengthAuthority | Self::SetProgramLengthRecipient => {
                &mut policy.loader_v4.allow_set_program_length
            }
            Self::Deploy => &mut policy.loader_v4.allow_deploy,
            Self::Retract => &mut policy.loader_v4.allow_retract,
            Self::TransferAuthorityCurrent | Self::TransferAuthorityNew => {
                &mut policy.loader_v4.allow_transfer_authority
            }
            Self::Finalize => &mut policy.loader_v4.allow_finalize,
        }
    }

    fn instruction(self, actor: &Pubkey) -> Instruction {
        let program = Pubkey::new_unique();
        let other = Pubkey::new_unique();
        match self {
            Self::Write => write(&program, actor, 0, vec![1, 2, 3]),
            Self::Copy => copy(&program, actor, &other, 0, 0, 1),
            // The drainage guard rejects a fee-payer authority paired with a foreign recipient
            // regardless of the flag, so this role has to name the actor in both slots to
            // isolate the flag as the only thing under test.
            Self::SetProgramLengthAuthority => set_program_length(&program, actor, 0, actor),
            Self::SetProgramLengthRecipient => set_program_length(&program, &other, 0, actor),
            Self::Deploy => deploy(&program, actor),
            Self::Retract => retract(&program, actor),
            Self::TransferAuthorityCurrent => transfer_authority(&program, actor, &other),
            Self::TransferAuthorityNew => transfer_authority(&program, &other, actor),
            Self::Finalize => finalize(&program, actor, &other),
        }
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(512))]

    #[test]
    fn fee_payer_role_gated_iff_flag_off(
        (role_idx, actor_is_fee_payer, flags) in role_and_flags::<LoaderV4Role>(),
    ) {
        assert_role_gated_iff_flag_off::<LoaderV4Role>(role_idx, actor_is_fee_payer, &flags)?;
    }
}
