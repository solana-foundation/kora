use super::{assert_role_gated_iff_flag_off, role_and_flags, validate, DrainRole};
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
    AssignWithSeedDecoy,
    Allocate,
    AllocateWithSeed,
    AllocateWithSeedDecoy,
    CreateAccountPayer,
    CreateAccountWithSeedPayer,
    CreateAccountWithSeedBase,
    CreateAccountAllowPrefundPayer,
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
        Self::AssignWithSeedDecoy,
        Self::Allocate,
        Self::AllocateWithSeed,
        Self::AllocateWithSeedDecoy,
        Self::CreateAccountPayer,
        Self::CreateAccountWithSeedPayer,
        Self::CreateAccountWithSeedBase,
        Self::CreateAccountAllowPrefundPayer,
        Self::NonceInitialize,
        Self::NonceAdvance,
        Self::NonceAuthorize,
        Self::NonceWithdraw,
    ];

    fn allowed_programs() -> Vec<String> {
        vec![SYSTEM_PROGRAM_ID.to_string(), spl_token_interface::id().to_string()]
    }

    fn flag(self, policy: &mut FeePayerPolicy) -> &mut bool {
        match self {
            Self::Transfer | Self::TransferWithSeed => &mut policy.system.allow_transfer,
            Self::Assign | Self::AssignWithSeed | Self::AssignWithSeedDecoy => {
                &mut policy.system.allow_assign
            }
            Self::Allocate | Self::AllocateWithSeed | Self::AllocateWithSeedDecoy => {
                &mut policy.system.allow_allocate
            }
            Self::CreateAccountPayer
            | Self::CreateAccountWithSeedPayer
            | Self::CreateAccountWithSeedBase
            | Self::CreateAccountAllowPrefundPayer => &mut policy.system.allow_create_account,
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
            Self::AssignWithSeedDecoy => with_decoy_at_base_meta(
                assign_with_seed(&Pubkey::new_unique(), actor, "seed", &SYSTEM_PROGRAM_ID),
                actor,
            ),
            Self::Allocate => allocate(actor, 8),
            Self::AllocateWithSeed => {
                allocate_with_seed(&Pubkey::new_unique(), actor, "seed", 8, &SYSTEM_PROGRAM_ID)
            }
            Self::AllocateWithSeedDecoy => with_decoy_at_base_meta(
                allocate_with_seed(&Pubkey::new_unique(), actor, "seed", 8, &SYSTEM_PROGRAM_ID),
                actor,
            ),
            Self::CreateAccountPayer => {
                create_account(actor, &Pubkey::new_unique(), 1_000, 8, &SYSTEM_PROGRAM_ID)
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

fn with_decoy_at_base_meta(mut instruction: Instruction, base: &Pubkey) -> Instruction {
    instruction.accounts[1] = AccountMeta::new_readonly(Pubkey::new_unique(), true);
    instruction.accounts.push(AccountMeta::new_readonly(*base, true));
    instruction
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

fn fee_payer_as_created_account(fee_payer: &Pubkey) -> [Instruction; 3] {
    [
        create_account(&Pubkey::new_unique(), fee_payer, 1_000, 8, &SYSTEM_PROGRAM_ID),
        create_account_with_seed(
            &Pubkey::new_unique(),
            fee_payer,
            &Pubkey::new_unique(),
            "seed",
            1_000,
            8,
            &SYSTEM_PROGRAM_ID,
        ),
        create_account_allow_prefund(fee_payer, &Pubkey::new_unique()),
    ]
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(128))]

    #[test]
    fn fee_payer_as_created_account_always_denied(
        flags in prop::collection::vec(any::<bool>(), SystemRole::ROLES.len()),
    ) {
        let mut policy = FeePayerPolicy::default();
        for (role, allowed) in SystemRole::ROLES.iter().zip(&flags) {
            *role.flag(&mut policy) = *allowed;
        }
        let fee_payer = Pubkey::new_unique();
        for ix in fee_payer_as_created_account(&fee_payer) {
            let result = validate::<SystemRole>(policy.clone(), fee_payer, ix);
            prop_assert!(result.is_err(), "fee payer as created account must be denied");
        }
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(128))]

    #[test]
    fn fee_payer_assigned_to_foreign_owner_always_denied(
        flags in prop::collection::vec(any::<bool>(), SystemRole::ROLES.len()),
    ) {
        let mut policy = FeePayerPolicy::default();
        for (role, allowed) in SystemRole::ROLES.iter().zip(&flags) {
            *role.flag(&mut policy) = *allowed;
        }
        let fee_payer = Pubkey::new_unique();
        let ix = assign(&fee_payer, &spl_token_interface::id());
        let result = validate::<SystemRole>(policy, fee_payer, ix);
        prop_assert!(result.is_err(), "fee payer reassigned to a foreign owner must be denied");
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(128))]

    #[test]
    fn fee_payer_as_nonce_account_always_denied(
        flags in prop::collection::vec(any::<bool>(), SystemRole::ROLES.len()),
    ) {
        let mut policy = FeePayerPolicy::default();
        for (role, allowed) in SystemRole::ROLES.iter().zip(&flags) {
            *role.flag(&mut policy) = *allowed;
        }
        let fee_payer = Pubkey::new_unique();
        let ix = create_nonce_account(&Pubkey::new_unique(), &fee_payer, &fee_payer, 1_000_000)
            .swap_remove(1);
        let result = validate::<SystemRole>(policy, fee_payer, ix);
        prop_assert!(result.is_err(), "fee payer initialized as a nonce account must be denied");
    }
}
