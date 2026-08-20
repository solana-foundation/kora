use super::{assert_role_gated_iff_flag_off, role_and_flags, DrainRole};
use crate::config::FeePayerPolicy;
use proptest::prelude::*;
use solana_sdk::{instruction::Instruction, pubkey::Pubkey};
use solana_system_interface::{
    instruction::{
        advance_nonce_account, allocate, assign, authorize_nonce_account, create_account,
        create_nonce_account, transfer, withdraw_nonce_account,
    },
    program::ID as SYSTEM_PROGRAM_ID,
};

#[derive(Debug, Clone, Copy)]
enum SystemRole {
    Transfer,
    Assign,
    Allocate,
    CreateAccountPayer,
    CreateAccountNewAccount,
    NonceInitialize,
    NonceAdvance,
    NonceAuthorize,
    NonceWithdraw,
}

impl DrainRole for SystemRole {
    const ROLES: &'static [Self] = &[
        Self::Transfer,
        Self::Assign,
        Self::Allocate,
        Self::CreateAccountPayer,
        Self::CreateAccountNewAccount,
        Self::NonceInitialize,
        Self::NonceAdvance,
        Self::NonceAuthorize,
        Self::NonceWithdraw,
    ];

    fn allowed_programs() -> Vec<String> {
        vec![SYSTEM_PROGRAM_ID.to_string()]
    }

    fn flag(self, policy: &mut FeePayerPolicy) -> &mut bool {
        match self {
            Self::Transfer => &mut policy.system.allow_transfer,
            Self::Assign => &mut policy.system.allow_assign,
            Self::Allocate => &mut policy.system.allow_allocate,
            Self::CreateAccountPayer | Self::CreateAccountNewAccount => {
                &mut policy.system.allow_create_account
            }
            Self::NonceInitialize => &mut policy.system.nonce.allow_initialize,
            Self::NonceAdvance => &mut policy.system.nonce.allow_advance,
            Self::NonceAuthorize => &mut policy.system.nonce.allow_authorize,
            Self::NonceWithdraw => &mut policy.system.nonce.allow_withdraw,
        }
    }

    fn instruction(self, actor: &Pubkey) -> Instruction {
        match self {
            Self::Transfer => transfer(actor, &Pubkey::new_unique(), 1_000),
            Self::Assign => assign(actor, &SYSTEM_PROGRAM_ID),
            Self::Allocate => allocate(actor, 8),
            Self::CreateAccountPayer => {
                create_account(actor, &Pubkey::new_unique(), 1_000, 8, &SYSTEM_PROGRAM_ID)
            }
            Self::CreateAccountNewAccount => {
                create_account(&Pubkey::new_unique(), actor, 1_000, 8, &SYSTEM_PROGRAM_ID)
            }
            Self::NonceInitialize => {
                create_nonce_account(&Pubkey::new_unique(), &Pubkey::new_unique(), actor, 1_000_000)
                    .swap_remove(1)
            }
            Self::NonceAdvance => advance_nonce_account(&Pubkey::new_unique(), actor),
            Self::NonceAuthorize => {
                authorize_nonce_account(&Pubkey::new_unique(), actor, &Pubkey::new_unique())
            }
            Self::NonceWithdraw => {
                withdraw_nonce_account(&Pubkey::new_unique(), actor, &Pubkey::new_unique(), 1_000)
            }
        }
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(512))]

    #[test]
    fn fee_payer_role_gated_iff_flag_off(
        (role_idx, actor_is_fee_payer, flags) in role_and_flags::<SystemRole>(),
    ) {
        assert_role_gated_iff_flag_off::<SystemRole>(role_idx, actor_is_fee_payer, &flags)?;
    }
}
