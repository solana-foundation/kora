use super::{assert_role_gated_iff_flag_off, role_and_flags, DrainRole};
use crate::{
    config::FeePayerPolicy,
    constant::instruction_indexes::system_create_account_allow_prefund::DISCRIMINATOR,
};
use proptest::prelude::*;
use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
};
use solana_system_interface::{
    instruction::{
        advance_nonce_account, allocate, allocate_with_seed, assign, assign_with_seed,
        authorize_nonce_account, create_account, create_account_with_seed, create_nonce_account,
        transfer, transfer_with_seed, withdraw_nonce_account,
    },
    program::ID as SYSTEM_PROGRAM_ID,
};

#[derive(Debug, Clone, Copy)]
enum SystemRole {
    Transfer,
    TransferWithSeed,
    Assign,
    AssignWithSeed,
    Allocate,
    AllocateWithSeed,
    CreateAccountPayer,
    CreateAccountNewAccount,
    CreateAccountWithSeedPayer,
    CreateAccountWithSeedNewAccount,
    CreateAccountWithSeedBase,
    CreateAccountAllowPrefundPayer,
    CreateAccountAllowPrefundNewAccount,
    NonceInitialize,
    NonceAdvance,
    NonceAuthorize,
    NonceWithdraw,
}

impl DrainRole for SystemRole {
    const ROLES: &'static [Self] = &[
        Self::Transfer,
        Self::TransferWithSeed,
        Self::Assign,
        Self::AssignWithSeed,
        Self::Allocate,
        Self::AllocateWithSeed,
        Self::CreateAccountPayer,
        Self::CreateAccountNewAccount,
        Self::CreateAccountWithSeedPayer,
        Self::CreateAccountWithSeedNewAccount,
        Self::CreateAccountWithSeedBase,
        Self::CreateAccountAllowPrefundPayer,
        Self::CreateAccountAllowPrefundNewAccount,
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
            Self::Transfer | Self::TransferWithSeed => &mut policy.system.allow_transfer,
            Self::Assign | Self::AssignWithSeed => &mut policy.system.allow_assign,
            Self::Allocate | Self::AllocateWithSeed => &mut policy.system.allow_allocate,
            Self::CreateAccountPayer
            | Self::CreateAccountNewAccount
            | Self::CreateAccountWithSeedPayer
            | Self::CreateAccountWithSeedNewAccount
            | Self::CreateAccountWithSeedBase
            | Self::CreateAccountAllowPrefundPayer
            | Self::CreateAccountAllowPrefundNewAccount => &mut policy.system.allow_create_account,
            Self::NonceInitialize => &mut policy.system.nonce.allow_initialize,
            Self::NonceAdvance => &mut policy.system.nonce.allow_advance,
            Self::NonceAuthorize => &mut policy.system.nonce.allow_authorize,
            Self::NonceWithdraw => &mut policy.system.nonce.allow_withdraw,
        }
    }

    fn instruction(self, actor: &Pubkey) -> Instruction {
        match self {
            Self::Transfer => transfer(actor, &Pubkey::new_unique(), 1_000),
            Self::TransferWithSeed => transfer_with_seed(
                &Pubkey::new_unique(),
                actor,
                "seed".to_string(),
                &SYSTEM_PROGRAM_ID,
                &Pubkey::new_unique(),
                1_000,
            ),
            Self::Assign => assign(actor, &SYSTEM_PROGRAM_ID),
            Self::AssignWithSeed => {
                assign_with_seed(&Pubkey::new_unique(), actor, "seed", &SYSTEM_PROGRAM_ID)
            }
            Self::Allocate => allocate(actor, 8),
            Self::AllocateWithSeed => {
                allocate_with_seed(&Pubkey::new_unique(), actor, "seed", 8, &SYSTEM_PROGRAM_ID)
            }
            Self::CreateAccountPayer => {
                create_account(actor, &Pubkey::new_unique(), 1_000, 8, &SYSTEM_PROGRAM_ID)
            }
            Self::CreateAccountNewAccount => {
                create_account(&Pubkey::new_unique(), actor, 1_000, 8, &SYSTEM_PROGRAM_ID)
            }
            Self::CreateAccountWithSeedPayer => create_account_with_seed(
                actor,
                &Pubkey::new_unique(),
                &Pubkey::new_unique(),
                "seed",
                1_000,
                8,
                &SYSTEM_PROGRAM_ID,
            ),
            Self::CreateAccountWithSeedNewAccount => create_account_with_seed(
                &Pubkey::new_unique(),
                actor,
                &Pubkey::new_unique(),
                "seed",
                1_000,
                8,
                &SYSTEM_PROGRAM_ID,
            ),
            Self::CreateAccountWithSeedBase => create_account_with_seed(
                &Pubkey::new_unique(),
                &Pubkey::new_unique(),
                actor,
                "seed",
                1_000,
                8,
                &SYSTEM_PROGRAM_ID,
            ),
            Self::CreateAccountAllowPrefundPayer => {
                create_account_allow_prefund(&Pubkey::new_unique(), actor)
            }
            Self::CreateAccountAllowPrefundNewAccount => {
                create_account_allow_prefund(actor, &Pubkey::new_unique())
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

fn create_account_allow_prefund(new_account: &Pubkey, funder: &Pubkey) -> Instruction {
    Instruction {
        program_id: SYSTEM_PROGRAM_ID,
        accounts: vec![AccountMeta::new(*new_account, true), AccountMeta::new(*funder, true)],
        data: bincode::serialize(&(DISCRIMINATOR, 1_000u64, 8u64, SYSTEM_PROGRAM_ID)).unwrap(),
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
