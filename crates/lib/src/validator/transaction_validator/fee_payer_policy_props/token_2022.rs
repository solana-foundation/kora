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
        approve, approve_checked, burn, burn_checked, close_account, freeze_account,
        initialize_account, initialize_account2, initialize_account3, initialize_mint,
        initialize_mint2, initialize_mint_close_authority, initialize_multisig,
        initialize_permanent_delegate, mint_to, mint_to_checked, revoke, set_authority,
        thaw_account, transfer_checked, unwrap_lamports, withdraw_excess_lamports, AuthorityType,
    },
};

#[derive(Debug, Clone, Copy)]
enum Token2022Role {
    Transfer,
    TransferChecked,
    TransferCheckedWithFee,
    TransferMultisigSigner,
    Burn,
    BurnChecked,
    BurnMultisigSigner,
    CloseAccount,
    CloseAccountMultisigSigner,
    Approve,
    ApproveChecked,
    ApproveMultisigSigner,
    Revoke,
    RevokeMultisigSigner,
    SetAuthority,
    SetAuthorityMultisigSigner,
    MintTo,
    MintToChecked,
    MintToMultisigSigner,
    InitializeMint,
    InitializeMint2,
    InitializeAccount,
    InitializeAccount2,
    InitializeAccount3,
    InitializeMultisig,
    FreezeAccount,
    FreezeAccountMultisigSigner,
    Pause,
    PauseMultisigSigner,
    ThawAccount,
    ThawAccountMultisigSigner,
    Resume,
    ResumeMultisigSigner,
    WithdrawExcessLamports,
    WithdrawExcessLamportsMultisigSigner,
    UnwrapLamports,
    UnwrapLamportsMultisigSigner,
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
    UpdateMetadataPointerMultisigSigner,
    UpdateGroupPointer,
    UpdateGroupMemberPointer,
    UpdateScaledUiAmountMultiplier,
}

impl DrainRole for Token2022Role {
    const ROLES: &'static [Self] = &[
        Self::Transfer,
        Self::TransferChecked,
        Self::TransferCheckedWithFee,
        Self::TransferMultisigSigner,
        Self::Burn,
        Self::BurnChecked,
        Self::BurnMultisigSigner,
        Self::CloseAccount,
        Self::CloseAccountMultisigSigner,
        Self::Approve,
        Self::ApproveChecked,
        Self::ApproveMultisigSigner,
        Self::Revoke,
        Self::RevokeMultisigSigner,
        Self::SetAuthority,
        Self::SetAuthorityMultisigSigner,
        Self::MintTo,
        Self::MintToChecked,
        Self::MintToMultisigSigner,
        Self::InitializeMint,
        Self::InitializeMint2,
        Self::InitializeAccount,
        Self::InitializeAccount2,
        Self::InitializeAccount3,
        Self::InitializeMultisig,
        Self::FreezeAccount,
        Self::FreezeAccountMultisigSigner,
        Self::Pause,
        Self::PauseMultisigSigner,
        Self::ThawAccount,
        Self::ThawAccountMultisigSigner,
        Self::Resume,
        Self::ResumeMultisigSigner,
        Self::WithdrawExcessLamports,
        Self::WithdrawExcessLamportsMultisigSigner,
        Self::UnwrapLamports,
        Self::UnwrapLamportsMultisigSigner,
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
        Self::UpdateMetadataPointerMultisigSigner,
        Self::UpdateGroupPointer,
        Self::UpdateGroupMemberPointer,
        Self::UpdateScaledUiAmountMultiplier,
    ];

    fn allowed_programs() -> Vec<String> {
        vec![token_2022_id().to_string()]
    }

    fn flag(self, policy: &mut FeePayerPolicy) -> &mut bool {
        match self {
            Self::Transfer
            | Self::TransferChecked
            | Self::TransferCheckedWithFee
            | Self::TransferMultisigSigner => &mut policy.token_2022.allow_transfer,
            Self::Burn | Self::BurnChecked | Self::BurnMultisigSigner => {
                &mut policy.token_2022.allow_burn
            }
            Self::CloseAccount | Self::CloseAccountMultisigSigner => {
                &mut policy.token_2022.allow_close_account
            }
            Self::Approve | Self::ApproveChecked | Self::ApproveMultisigSigner => {
                &mut policy.token_2022.allow_approve
            }
            Self::Revoke | Self::RevokeMultisigSigner => &mut policy.token_2022.allow_revoke,
            Self::SetAuthority | Self::SetAuthorityMultisigSigner => {
                &mut policy.token_2022.allow_set_authority
            }
            Self::MintTo | Self::MintToChecked | Self::MintToMultisigSigner => {
                &mut policy.token_2022.allow_mint_to
            }
            Self::InitializeMint | Self::InitializeMint2 => {
                &mut policy.token_2022.allow_initialize_mint
            }
            Self::InitializeAccount | Self::InitializeAccount2 | Self::InitializeAccount3 => {
                &mut policy.token_2022.allow_initialize_account
            }
            Self::InitializeMultisig => &mut policy.token_2022.allow_initialize_multisig,
            Self::FreezeAccount
            | Self::FreezeAccountMultisigSigner
            | Self::Pause
            | Self::PauseMultisigSigner => &mut policy.token_2022.allow_freeze_account,
            Self::ThawAccount
            | Self::ThawAccountMultisigSigner
            | Self::Resume
            | Self::ResumeMultisigSigner => &mut policy.token_2022.allow_thaw_account,
            Self::WithdrawExcessLamports | Self::WithdrawExcessLamportsMultisigSigner => {
                &mut policy.token_2022.allow_withdraw_excess_lamports
            }
            Self::UnwrapLamports | Self::UnwrapLamportsMultisigSigner => {
                &mut policy.token_2022.allow_unwrap_lamports
            }
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
            | Self::UpdateMetadataPointerMultisigSigner
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
        let multisig = Pubkey::new_unique();
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
            Self::TransferChecked => {
                transfer_checked(&program, &account, &mint, &other, actor, &[], 1, 0).unwrap()
            }
            Self::TransferCheckedWithFee => transfer_fee::instruction::transfer_checked_with_fee(
                &program,
                &account,
                &mint,
                &other,
                actor,
                &[],
                1,
                0,
                0,
            )
            .unwrap(),
            #[allow(deprecated)]
            Self::TransferMultisigSigner => spl_token_2022_interface::instruction::transfer(
                &program,
                &account,
                &other,
                &multisig,
                &[actor],
                1,
            )
            .unwrap(),
            Self::Burn => burn(&program, &account, &mint, actor, &[], 1).unwrap(),
            Self::BurnChecked => burn_checked(&program, &account, &mint, actor, &[], 1, 0).unwrap(),
            Self::BurnMultisigSigner => {
                burn(&program, &account, &mint, &multisig, &[actor], 1).unwrap()
            }
            Self::CloseAccount => close_account(&program, &account, &other, actor, &[]).unwrap(),
            Self::CloseAccountMultisigSigner => {
                close_account(&program, &account, &other, &multisig, &[actor]).unwrap()
            }
            Self::Approve => approve(&program, &account, &other, actor, &[], 1).unwrap(),
            Self::ApproveChecked => {
                approve_checked(&program, &account, &mint, &other, actor, &[], 1, 0).unwrap()
            }
            Self::ApproveMultisigSigner => {
                approve(&program, &account, &other, &multisig, &[actor], 1).unwrap()
            }
            Self::Revoke => revoke(&program, &account, actor, &[]).unwrap(),
            Self::RevokeMultisigSigner => revoke(&program, &account, &multisig, &[actor]).unwrap(),
            Self::SetAuthority => set_authority(
                &program,
                &account,
                Some(&other),
                AuthorityType::AccountOwner,
                actor,
                &[],
            )
            .unwrap(),
            Self::SetAuthorityMultisigSigner => set_authority(
                &program,
                &account,
                Some(&other),
                AuthorityType::AccountOwner,
                &multisig,
                &[actor],
            )
            .unwrap(),
            Self::MintTo => mint_to(&program, &mint, &account, actor, &[], 1).unwrap(),
            Self::MintToChecked => {
                mint_to_checked(&program, &mint, &account, actor, &[], 1, 0).unwrap()
            }
            Self::MintToMultisigSigner => {
                mint_to(&program, &mint, &account, &multisig, &[actor], 1).unwrap()
            }
            Self::InitializeMint => initialize_mint(&program, &mint, actor, None, 0).unwrap(),
            Self::InitializeMint2 => initialize_mint2(&program, &mint, actor, None, 0).unwrap(),
            Self::InitializeAccount => {
                initialize_account(&program, &account, &mint, actor).unwrap()
            }
            Self::InitializeAccount2 => {
                initialize_account2(&program, &account, &mint, actor).unwrap()
            }
            Self::InitializeAccount3 => {
                initialize_account3(&program, &account, &mint, actor).unwrap()
            }
            Self::InitializeMultisig => {
                initialize_multisig(&program, &account, &[actor, &other], 1).unwrap()
            }
            Self::FreezeAccount => freeze_account(&program, &account, &mint, actor, &[]).unwrap(),
            Self::FreezeAccountMultisigSigner => {
                freeze_account(&program, &account, &mint, &multisig, &[actor]).unwrap()
            }
            Self::Pause => pausable::instruction::pause(&program, &mint, actor, &[]).unwrap(),
            Self::PauseMultisigSigner => {
                pausable::instruction::pause(&program, &mint, &multisig, &[actor]).unwrap()
            }
            Self::ThawAccount => thaw_account(&program, &account, &mint, actor, &[]).unwrap(),
            Self::ThawAccountMultisigSigner => {
                thaw_account(&program, &account, &mint, &multisig, &[actor]).unwrap()
            }
            Self::Resume => pausable::instruction::resume(&program, &mint, actor, &[]).unwrap(),
            Self::ResumeMultisigSigner => {
                pausable::instruction::resume(&program, &mint, &multisig, &[actor]).unwrap()
            }
            Self::WithdrawExcessLamports => {
                withdraw_excess_lamports(&program, &account, &other, actor, &[]).unwrap()
            }
            Self::WithdrawExcessLamportsMultisigSigner => {
                withdraw_excess_lamports(&program, &account, &other, &multisig, &[actor]).unwrap()
            }
            Self::UnwrapLamports => {
                unwrap_lamports(&program, &account, &other, actor, &[], None).unwrap()
            }
            Self::UnwrapLamportsMultisigSigner => {
                unwrap_lamports(&program, &account, &other, &multisig, &[actor], None).unwrap()
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
            Self::UpdateMetadataPointerMultisigSigner => {
                metadata_pointer::instruction::update(&program, &mint, &multisig, &[actor], None)
                    .unwrap()
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
