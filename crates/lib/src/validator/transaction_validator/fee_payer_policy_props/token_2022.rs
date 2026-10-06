use super::{assert_role_gated_iff_flag_off, role_and_flags, DrainRole};
use crate::config::FeePayerPolicy;
use proptest::prelude::*;
use solana_sdk::{instruction::Instruction, pubkey::Pubkey};
use spl_token_2022_interface::{
    extension::{
        group_member_pointer, group_pointer, interest_bearing_mint, metadata_pointer, pausable,
        scaled_ui_amount, transfer_fee,
    },
    id as token_2022_id,
    instruction::{
        approve, burn, close_account, freeze_account, initialize_account, initialize_mint,
        initialize_mint_close_authority, initialize_multisig, initialize_permanent_delegate,
        mint_to, revoke, set_authority, thaw_account, unwrap_lamports, withdraw_excess_lamports,
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
    Pause,
    ThawAccount,
    Resume,
    WithdrawExcessLamports,
    UnwrapLamports,
    InitializeMintCloseAuthority,
    InitializePermanentDelegate,
    InitializeTransferFeeConfigAuthority,
    InitializeTransferFeeWithdrawWithheldAuthority,
    InitializeInterestBearingRateAuthority,
    InitializeMetadataPointerAuthority,
    InitializeGroupPointerAuthority,
    InitializeGroupMemberPointerAuthority,
    InitializeScaledUiAmountAuthority,
    InitializePausableAuthority,
    SetTransferFee,
    WithdrawWithheldTokensFromMint,
    WithdrawWithheldTokensFromAccounts,
    UpdateInterestBearingRate,
    UpdateMetadataPointer,
    UpdateGroupPointer,
    UpdateGroupMemberPointer,
    UpdateScaledUiAmountMultiplier,
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
        Self::Pause,
        Self::ThawAccount,
        Self::Resume,
        Self::WithdrawExcessLamports,
        Self::UnwrapLamports,
        Self::InitializeMintCloseAuthority,
        Self::InitializePermanentDelegate,
        Self::InitializeTransferFeeConfigAuthority,
        Self::InitializeTransferFeeWithdrawWithheldAuthority,
        Self::InitializeInterestBearingRateAuthority,
        Self::InitializeMetadataPointerAuthority,
        Self::InitializeGroupPointerAuthority,
        Self::InitializeGroupMemberPointerAuthority,
        Self::InitializeScaledUiAmountAuthority,
        Self::InitializePausableAuthority,
        Self::SetTransferFee,
        Self::WithdrawWithheldTokensFromMint,
        Self::WithdrawWithheldTokensFromAccounts,
        Self::UpdateInterestBearingRate,
        Self::UpdateMetadataPointer,
        Self::UpdateGroupPointer,
        Self::UpdateGroupMemberPointer,
        Self::UpdateScaledUiAmountMultiplier,
    ];

    fn allowed_programs() -> Vec<String> {
        vec![token_2022_id().to_string()]
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
            Self::WithdrawExcessLamports => &mut policy.token_2022.allow_withdraw_excess_lamports,
            Self::UnwrapLamports => &mut policy.token_2022.allow_unwrap_lamports,
            Self::InitializeMintCloseAuthority
            | Self::InitializePermanentDelegate
            | Self::InitializeTransferFeeConfigAuthority
            | Self::InitializeTransferFeeWithdrawWithheldAuthority
            | Self::InitializeInterestBearingRateAuthority
            | Self::InitializeMetadataPointerAuthority
            | Self::InitializeGroupPointerAuthority
            | Self::InitializeGroupMemberPointerAuthority
            | Self::InitializeScaledUiAmountAuthority
            | Self::InitializePausableAuthority => {
                &mut policy.token_2022.allow_initialize_extension_authority
            }
            Self::SetTransferFee
            | Self::WithdrawWithheldTokensFromMint
            | Self::WithdrawWithheldTokensFromAccounts
            | Self::UpdateInterestBearingRate
            | Self::UpdateMetadataPointer
            | Self::UpdateGroupPointer
            | Self::UpdateGroupMemberPointer
            | Self::UpdateScaledUiAmountMultiplier => {
                &mut policy.token_2022.allow_update_extension_authority
            }
        }
    }

    fn instruction(self, actor: &Pubkey) -> Instruction {
        let program = token_2022_id();
        let account = Pubkey::new_unique();
        let mint = Pubkey::new_unique();
        let other = Pubkey::new_unique();
        match self {
            #[allow(deprecated)]
            Self::Transfer => spl_token_2022_interface::instruction::transfer(
                &program,
                &account,
                &other,
                actor,
                &[],
                1,
            )
            .unwrap(),
            Self::Burn => burn(&program, &account, &mint, actor, &[], 1).unwrap(),
            Self::CloseAccount => close_account(&program, &account, &other, actor, &[]).unwrap(),
            Self::Approve => approve(&program, &account, &other, actor, &[], 1).unwrap(),
            Self::Revoke => revoke(&program, &account, actor, &[]).unwrap(),
            Self::SetAuthority => set_authority(
                &program,
                &account,
                Some(&other),
                AuthorityType::AccountOwner,
                actor,
                &[],
            )
            .unwrap(),
            Self::MintTo => mint_to(&program, &mint, &account, actor, &[], 1).unwrap(),
            Self::InitializeMint => initialize_mint(&program, &mint, actor, None, 0).unwrap(),
            Self::InitializeAccount => {
                initialize_account(&program, &account, &mint, actor).unwrap()
            }
            Self::InitializeMultisig => {
                initialize_multisig(&program, &account, &[actor, &other], 1).unwrap()
            }
            Self::FreezeAccount => freeze_account(&program, &account, &mint, actor, &[]).unwrap(),
            Self::Pause => pausable::instruction::pause(&program, &mint, actor, &[]).unwrap(),
            Self::ThawAccount => thaw_account(&program, &account, &mint, actor, &[]).unwrap(),
            Self::Resume => pausable::instruction::resume(&program, &mint, actor, &[]).unwrap(),
            Self::WithdrawExcessLamports => {
                withdraw_excess_lamports(&program, &account, &other, actor, &[]).unwrap()
            }
            Self::UnwrapLamports => {
                unwrap_lamports(&program, &account, &other, actor, &[], None).unwrap()
            }
            Self::InitializeMintCloseAuthority => {
                initialize_mint_close_authority(&program, &mint, Some(actor)).unwrap()
            }
            Self::InitializePermanentDelegate => {
                initialize_permanent_delegate(&program, &mint, actor).unwrap()
            }
            Self::InitializeTransferFeeConfigAuthority => {
                transfer_fee::instruction::initialize_transfer_fee_config(
                    &program,
                    &mint,
                    Some(actor),
                    Some(&other),
                    0,
                    0,
                )
                .unwrap()
            }
            Self::InitializeTransferFeeWithdrawWithheldAuthority => {
                transfer_fee::instruction::initialize_transfer_fee_config(
                    &program,
                    &mint,
                    Some(&other),
                    Some(actor),
                    0,
                    0,
                )
                .unwrap()
            }
            Self::InitializeInterestBearingRateAuthority => {
                interest_bearing_mint::instruction::initialize(&program, &mint, Some(*actor), 0)
                    .unwrap()
            }
            Self::InitializeMetadataPointerAuthority => {
                metadata_pointer::instruction::initialize(&program, &mint, Some(*actor), None)
                    .unwrap()
            }
            Self::InitializeGroupPointerAuthority => {
                group_pointer::instruction::initialize(&program, &mint, Some(*actor), None).unwrap()
            }
            Self::InitializeGroupMemberPointerAuthority => {
                group_member_pointer::instruction::initialize(&program, &mint, Some(*actor), None)
                    .unwrap()
            }
            Self::InitializeScaledUiAmountAuthority => {
                scaled_ui_amount::instruction::initialize(&program, &mint, Some(*actor), 1.0)
                    .unwrap()
            }
            Self::InitializePausableAuthority => {
                pausable::instruction::initialize(&program, &mint, actor).unwrap()
            }
            Self::SetTransferFee => {
                transfer_fee::instruction::set_transfer_fee(&program, &mint, actor, &[], 0, 0)
                    .unwrap()
            }
            Self::WithdrawWithheldTokensFromMint => {
                transfer_fee::instruction::withdraw_withheld_tokens_from_mint(
                    &program,
                    &mint,
                    &other,
                    actor,
                    &[],
                )
                .unwrap()
            }
            Self::WithdrawWithheldTokensFromAccounts => {
                transfer_fee::instruction::withdraw_withheld_tokens_from_accounts(
                    &program,
                    &mint,
                    &other,
                    actor,
                    &[],
                    &[&account],
                )
                .unwrap()
            }
            Self::UpdateInterestBearingRate => {
                interest_bearing_mint::instruction::update_rate(&program, &mint, actor, &[], 0)
                    .unwrap()
            }
            Self::UpdateMetadataPointer => {
                metadata_pointer::instruction::update(&program, &mint, actor, &[], None).unwrap()
            }
            Self::UpdateGroupPointer => {
                group_pointer::instruction::update(&program, &mint, actor, &[], None).unwrap()
            }
            Self::UpdateGroupMemberPointer => {
                group_member_pointer::instruction::update(&program, &mint, actor, &[], None)
                    .unwrap()
            }
            Self::UpdateScaledUiAmountMultiplier => {
                scaled_ui_amount::instruction::update_multiplier(
                    &program,
                    &mint,
                    actor,
                    &[],
                    1.0,
                    0,
                )
                .unwrap()
            }
        }
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(512))]

    #[test]
    fn fee_payer_role_gated_iff_flag_off(
        (role_idx, actor_is_fee_payer, flags) in role_and_flags::<Token2022Role>(),
    ) {
        assert_role_gated_iff_flag_off::<Token2022Role>(role_idx, actor_is_fee_payer, &flags)?;
    }
}
