use super::{assert_role_gated_iff_flag_off, role_and_flags, DrainRole};
use crate::config::FeePayerPolicy;
use proptest::prelude::*;
use solana_address_lookup_table_interface::{
    instruction::{
        close_lookup_table, create_lookup_table, deactivate_lookup_table, extend_lookup_table,
        freeze_lookup_table,
    },
    program::ID as ADDRESS_LOOKUP_TABLE_PROGRAM_ID,
};
use solana_sdk::{instruction::Instruction, pubkey::Pubkey};

#[derive(Debug, Clone, Copy)]
enum AltRole {
    CreateAuthority,
    CreatePayer,
    ExtendAuthority,
    ExtendPayer,
    FreezeAuthority,
    DeactivateAuthority,
    CloseAuthority,
}

impl DrainRole for AltRole {
    const ROLES: &'static [Self] = &[
        Self::CreateAuthority,
        Self::CreatePayer,
        Self::ExtendAuthority,
        Self::ExtendPayer,
        Self::FreezeAuthority,
        Self::DeactivateAuthority,
        Self::CloseAuthority,
    ];

    fn allowed_programs() -> Vec<String> {
        vec![ADDRESS_LOOKUP_TABLE_PROGRAM_ID.to_string()]
    }

    fn flag(self, policy: &mut FeePayerPolicy) -> &mut bool {
        match self {
            Self::CreateAuthority | Self::CreatePayer => &mut policy.alt.allow_create,
            Self::ExtendAuthority | Self::ExtendPayer => &mut policy.alt.allow_extend,
            Self::FreezeAuthority => &mut policy.alt.allow_freeze,
            Self::DeactivateAuthority => &mut policy.alt.allow_deactivate,
            Self::CloseAuthority => &mut policy.alt.allow_close,
        }
    }

    fn instruction(self, actor: &Pubkey) -> Instruction {
        match self {
            Self::CreateAuthority => create_lookup_table(*actor, Pubkey::new_unique(), 42).0,
            Self::CreatePayer => create_lookup_table(Pubkey::new_unique(), *actor, 42).0,
            Self::ExtendAuthority => extend_lookup_table(
                Pubkey::new_unique(),
                *actor,
                Some(Pubkey::new_unique()),
                vec![Pubkey::new_unique()],
            ),
            Self::ExtendPayer => extend_lookup_table(
                Pubkey::new_unique(),
                Pubkey::new_unique(),
                Some(*actor),
                vec![Pubkey::new_unique()],
            ),
            Self::FreezeAuthority => freeze_lookup_table(Pubkey::new_unique(), *actor),
            Self::DeactivateAuthority => deactivate_lookup_table(Pubkey::new_unique(), *actor),
            Self::CloseAuthority => {
                close_lookup_table(Pubkey::new_unique(), *actor, Pubkey::new_unique())
            }
        }
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(512))]

    #[test]
    fn fee_payer_role_gated_iff_flag_off(
        (role_idx, actor_is_fee_payer, flags) in role_and_flags::<AltRole>(),
    ) {
        assert_role_gated_iff_flag_off::<AltRole>(role_idx, actor_is_fee_payer, &flags)?;
    }
}
