use super::{assert_role_gated_iff_flag_off, role_and_flags, DrainRole, TransactionValidator};
use crate::{
    config::FeePayerPolicy,
    constant::instruction_indexes::system_create_account_allow_prefund::DISCRIMINATOR,
    oracle::PriceSource, tests::config_mock::ConfigMockBuilder, transaction::TransactionUtil,
};
use proptest::prelude::*;
use solana_message::{Message, VersionedMessage};
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
use spl_associated_token_account_interface::instruction::{
    create_associated_token_account, create_associated_token_account_idempotent,
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

#[derive(Debug, Clone, Copy)]
enum Creation {
    CreateAccount,
    CreateAccountWithSeed,
    CreateAccountAllowPrefund,
    AtaCreate,
    AtaCreateIdempotent,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum Placement {
    TopLevel,
    InnerUnderListed,
    InnerUnderUnlisted,
}

const CREATIONS: &[Creation] = &[
    Creation::CreateAccount,
    Creation::CreateAccountWithSeed,
    Creation::CreateAccountAllowPrefund,
    Creation::AtaCreate,
    Creation::AtaCreateIdempotent,
];

const PLACEMENTS: &[Placement] =
    &[Placement::TopLevel, Placement::InnerUnderListed, Placement::InnerUnderUnlisted];

fn creation_funded_by(kind: Creation, funder: &Pubkey) -> Instruction {
    match kind {
        Creation::CreateAccount => {
            create_account(funder, &Pubkey::new_unique(), 1_000, 8, &SYSTEM_PROGRAM_ID)
        }
        Creation::CreateAccountWithSeed => create_account_with_seed(
            funder,
            &Pubkey::new_unique(),
            &Pubkey::new_unique(),
            "seed",
            1_000,
            8,
            &SYSTEM_PROGRAM_ID,
        ),
        Creation::CreateAccountAllowPrefund => {
            create_account_allow_prefund(&Pubkey::new_unique(), funder)
        }
        Creation::AtaCreate => create_associated_token_account(
            funder,
            &Pubkey::new_unique(),
            &Pubkey::new_unique(),
            &spl_token_interface::id(),
        ),
        Creation::AtaCreateIdempotent => create_associated_token_account_idempotent(
            funder,
            &Pubkey::new_unique(),
            &Pubkey::new_unique(),
            &spl_token_interface::id(),
        ),
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(512))]

    #[test]
    fn create_account_only_via_gates_fee_payer_funded_creations(
        kind in prop::sample::select(CREATIONS),
        placement in prop::sample::select(PLACEMENTS),
        funder_is_fee_payer in any::<bool>(),
        list_set in any::<bool>(),
        allow_create_account in any::<bool>(),
    ) {
        let fee_payer = Pubkey::new_unique();
        let listed = Pubkey::new_unique();
        let unlisted = Pubkey::new_unique();
        let funder = if funder_is_fee_payer { fee_payer } else { Pubkey::new_unique() };

        let mut policy = FeePayerPolicy::default();
        policy.system.allow_create_account = allow_create_account;
        if list_set {
            policy.system.create_account_only_via = vec![listed.to_string()];
        }
        let config = ConfigMockBuilder::new()
            .with_price_source(PriceSource::Mock)
            .with_allowed_programs(vec![
                SYSTEM_PROGRAM_ID.to_string(),
                spl_associated_token_account_interface::program::id().to_string(),
                listed.to_string(),
                unlisted.to_string(),
            ])
            .with_max_allowed_lamports(1_000_000)
            .with_fee_payer_policy(policy)
            .build();
        let validator = TransactionValidator::new(&config, fee_payer).unwrap();

        let call = |program: Pubkey| Instruction { program_id: program, accounts: vec![], data: vec![] };
        let creation = creation_funded_by(kind, &funder);
        let mut top_level = vec![call(listed), call(unlisted)];
        let parent_index = match placement {
            Placement::TopLevel => {
                top_level.push(creation.clone());
                None
            }
            Placement::InnerUnderListed => Some(0),
            Placement::InnerUnderUnlisted => Some(1),
        };
        let message = VersionedMessage::Legacy(Message::new(&top_level, Some(&fee_payer)));
        let mut resolved =
            TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();
        if let Some(parent_index) = parent_index {
            resolved.push_inner_instruction(parent_index, creation).unwrap();
        }

        let result = validator.validate_fee_payer_usage(&config, &mut resolved);
        let expected_ok = !funder_is_fee_payer
            || (allow_create_account && (!list_set || placement == Placement::InnerUnderListed));
        prop_assert_eq!(
            result.is_ok(),
            expected_ok,
            "{:?} at {:?}, funder_is_fee_payer={}, list_set={}, allow_create_account={}: {:?}",
            kind, placement, funder_is_fee_payer, list_set, allow_create_account, result
        );
    }
}
