use super::{assert_role_gated_iff_flag_off, role_and_flags, DrainRole};
use crate::config::FeePayerPolicy;
use proptest::prelude::*;
use solana_sdk::{instruction::Instruction, pubkey::Pubkey};
use spl_token_2022_interface::{
    extension::{
        metadata_pointer::instruction as metadata_pointer_ix,
        pausable::instruction as pausable_ix,
        transfer_fee::instruction as transfer_fee_ix,
    },
    id as token_2022_program_id,
    instruction::{
        approve, burn, close_account, freeze_account, initialize_account3, initialize_mint2,
        initialize_multisig2, mint_to, revoke, set_authority, thaw_account, transfer_checked,
        AuthorityType,
    },
};

#[derive(Debug, Clone, Copy)]
enum Token2022Role {
    Transfer,
    Burn,
    CloseAccount,
    Approve,
    Revoke,
    SetAuthority,
    MintTo,
    InitializeMint,
    InitializeAccount,
    InitializeMultisig,
    FreezeAccount,
    ThawAccount,
    Pause,
    Resume,
    InitializeExtensionAuthorityMetadataPointer,
    InitializeExtensionAuthorityTransferFee,
    UpdateExtensionAuthorityMetadataPointer,
}

impl DrainRole for Token2022Role {
    const ROLES: &'static [Self] = &[
        Self::Transfer,
        Self::Burn,
        Self::CloseAccount,
        Self::Approve,
        Self::Revoke,
        Self::SetAuthority,
        Self::MintTo,
        Self::InitializeMint,
        Self::InitializeAccount,
        Self::InitializeMultisig,
        Self::FreezeAccount,
        Self::ThawAccount,
        Self::Pause,
        Self::Resume,
        Self::InitializeExtensionAuthorityMetadataPointer,
        Self::InitializeExtensionAuthorityTransferFee,
        Self::UpdateExtensionAuthorityMetadataPointer,
    ];

    fn allowed_programs() -> Vec<String> {
        vec![token_2022_program_id().to_string()]
    }

    fn flag(self, policy: &mut FeePayerPolicy) -> &mut bool {
        match self {
            Self::Transfer => &mut policy.token_2022.allow_transfer,
            Self::Burn => &mut policy.token_2022.allow_burn,
            Self::CloseAccount => &mut policy.token_2022.allow_close_account,
            Self::Approve => &mut policy.token_2022.allow_approve,
            Self::Revoke => &mut policy.token_2022.allow_revoke,
            Self::SetAuthority => &mut policy.token_2022.allow_set_authority,
            Self::MintTo => &mut policy.token_2022.allow_mint_to,
            Self::InitializeMint => &mut policy.token_2022.allow_initialize_mint,
            Self::InitializeAccount => &mut policy.token_2022.allow_initialize_account,
            Self::InitializeMultisig => &mut policy.token_2022.allow_initialize_multisig,
            Self::FreezeAccount | Self::Pause => &mut policy.token_2022.allow_freeze_account,
            Self::ThawAccount | Self::Resume => &mut policy.token_2022.allow_thaw_account,
            Self::InitializeExtensionAuthorityMetadataPointer
            | Self::InitializeExtensionAuthorityTransferFee => {
                &mut policy.token_2022.allow_initialize_extension_authority
            }
            Self::UpdateExtensionAuthorityMetadataPointer => {
                &mut policy.token_2022.allow_update_extension_authority
            }
        }
    }

    fn instruction(self, actor: &Pubkey) -> Instruction {
        let program_id = token_2022_program_id();
        let mint = Pubkey::new_unique();
        let account = Pubkey::new_unique();
        let other = Pubkey::new_unique();
        match self {
            Self::Transfer => transfer_checked(
                &program_id,
                &account,
                &mint,
                &other,
                actor,
                &[],
                1,
                6,
            )
            .expect("transfer_checked"),
            Self::Burn => burn(&program_id, &account, &mint, actor, &[], 1).expect("burn"),
            Self::CloseAccount => {
                close_account(&program_id, &account, actor, actor, &[]).expect("close_account")
            }
            Self::Approve => {
                approve(&program_id, &account, &other, actor, &[], 1).expect("approve")
            }
            Self::Revoke => revoke(&program_id, &account, actor, &[]).expect("revoke"),
            Self::SetAuthority => set_authority(
                &program_id,
                &account,
                Some(&other),
                AuthorityType::AccountOwner,
                actor,
                &[],
            )
            .expect("set_authority"),
            Self::MintTo => mint_to(&program_id, &mint, &account, actor, &[], 1).expect("mint_to"),
            Self::InitializeMint => {
                initialize_mint2(&program_id, &mint, actor, None, 6).expect("initialize_mint2")
            }
            Self::InitializeAccount => {
                initialize_account3(&program_id, &account, &mint, actor).expect("initialize_account3")
            }
            Self::InitializeMultisig => {
                initialize_multisig2(&program_id, &account, &[actor], 1).expect("initialize_multisig2")
            }
            Self::FreezeAccount => {
                freeze_account(&program_id, &account, &mint, actor, &[]).expect("freeze_account")
            }
            Self::ThawAccount => {
                thaw_account(&program_id, &account, &mint, actor, &[]).expect("thaw_account")
            }
            Self::Pause => pausable_ix::pause(&program_id, &mint, actor, &[]).expect("pause"),
            Self::Resume => pausable_ix::resume(&program_id, &mint, actor, &[]).expect("resume"),
            Self::InitializeExtensionAuthorityMetadataPointer => {
                metadata_pointer_ix::initialize(&program_id, &mint, Some(*actor), Some(other))
                    .expect("metadata_pointer initialize")
            }
            Self::InitializeExtensionAuthorityTransferFee => {
                transfer_fee_ix::initialize_transfer_fee_config(
                    &program_id,
                    &mint,
                    Some(actor),
                    Some(&other),
                    100,
                    1_000,
                )
                .expect("transfer_fee initialize")
            }
            Self::UpdateExtensionAuthorityMetadataPointer => {
                metadata_pointer_ix::update(&program_id, &mint, actor, &[], Some(other))
                    .expect("metadata_pointer update")
            }
        }
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(256))]

    #[test]
    fn fee_payer_role_gated_iff_flag_off(
        (role_idx, actor_is_fee_payer, flags) in role_and_flags::<Token2022Role>(),
    ) {
        assert_role_gated_iff_flag_off::<Token2022Role>(role_idx, actor_is_fee_payer, &flags)?;
    }
}
