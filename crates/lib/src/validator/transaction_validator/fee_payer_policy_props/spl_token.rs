use super::{assert_role_gated_iff_flag_off, role_and_flags, DrainRole};
use crate::config::FeePayerPolicy;
use proptest::prelude::*;
use solana_sdk::{instruction::Instruction, pubkey::Pubkey};
use spl_token_interface::{
    id as spl_token_id,
    instruction::{
        approve, approve_checked, burn, burn_checked, close_account, freeze_account,
        initialize_account, initialize_account2, initialize_account3, initialize_mint,
        initialize_mint2, initialize_multisig, mint_to, mint_to_checked, revoke, set_authority,
        thaw_account, transfer, transfer_checked, unwrap_lamports, withdraw_excess_lamports,
        AuthorityType,
    },
};

#[derive(Debug, Clone, Copy)]
enum SplTokenRole {
    Transfer,
    TransferChecked,
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
    SetAuthorityCurrent,
    SetAuthorityNew,
    SetAuthorityMultisigSigner,
    MintTo,
    MintToChecked,
    MintToMultisigSigner,
    InitializeMintAuthority,
    InitializeMintFreezeAuthority,
    InitializeMint2,
    InitializeAccount,
    InitializeAccount2,
    InitializeAccount3,
    InitializeMultisig,
    FreezeAccount,
    FreezeAccountMultisigSigner,
    ThawAccount,
    ThawAccountMultisigSigner,
    WithdrawExcessLamports,
    WithdrawExcessLamportsMultisigSigner,
    UnwrapLamports,
    UnwrapLamportsMultisigSigner,
}

impl DrainRole for SplTokenRole {
    const ROLES: &'static [Self] = &[
        Self::Transfer,
        Self::TransferChecked,
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
        Self::SetAuthorityCurrent,
        Self::SetAuthorityNew,
        Self::SetAuthorityMultisigSigner,
        Self::MintTo,
        Self::MintToChecked,
        Self::MintToMultisigSigner,
        Self::InitializeMintAuthority,
        Self::InitializeMintFreezeAuthority,
        Self::InitializeMint2,
        Self::InitializeAccount,
        Self::InitializeAccount2,
        Self::InitializeAccount3,
        Self::InitializeMultisig,
        Self::FreezeAccount,
        Self::FreezeAccountMultisigSigner,
        Self::ThawAccount,
        Self::ThawAccountMultisigSigner,
        Self::WithdrawExcessLamports,
        Self::WithdrawExcessLamportsMultisigSigner,
        Self::UnwrapLamports,
        Self::UnwrapLamportsMultisigSigner,
    ];

    fn allowed_programs() -> Vec<String> {
        vec![spl_token_id().to_string()]
    }

    fn flag(self, policy: &mut FeePayerPolicy) -> &mut bool {
        match self {
            Self::Transfer | Self::TransferChecked | Self::TransferMultisigSigner => {
                &mut policy.spl_token.allow_transfer
            }
            Self::Burn | Self::BurnChecked | Self::BurnMultisigSigner => {
                &mut policy.spl_token.allow_burn
            }
            Self::CloseAccount | Self::CloseAccountMultisigSigner => {
                &mut policy.spl_token.allow_close_account
            }
            Self::Approve | Self::ApproveChecked | Self::ApproveMultisigSigner => {
                &mut policy.spl_token.allow_approve
            }
            Self::Revoke | Self::RevokeMultisigSigner => &mut policy.spl_token.allow_revoke,
            Self::SetAuthorityCurrent
            | Self::SetAuthorityNew
            | Self::SetAuthorityMultisigSigner => &mut policy.spl_token.allow_set_authority,
            Self::MintTo | Self::MintToChecked | Self::MintToMultisigSigner => {
                &mut policy.spl_token.allow_mint_to
            }
            Self::InitializeMintAuthority
            | Self::InitializeMintFreezeAuthority
            | Self::InitializeMint2 => &mut policy.spl_token.allow_initialize_mint,
            Self::InitializeAccount | Self::InitializeAccount2 | Self::InitializeAccount3 => {
                &mut policy.spl_token.allow_initialize_account
            }
            Self::InitializeMultisig => &mut policy.spl_token.allow_initialize_multisig,
            Self::FreezeAccount | Self::FreezeAccountMultisigSigner => {
                &mut policy.spl_token.allow_freeze_account
            }
            Self::ThawAccount | Self::ThawAccountMultisigSigner => {
                &mut policy.spl_token.allow_thaw_account
            }
            Self::WithdrawExcessLamports | Self::WithdrawExcessLamportsMultisigSigner => {
                &mut policy.spl_token.allow_withdraw_excess_lamports
            }
            Self::UnwrapLamports | Self::UnwrapLamportsMultisigSigner => {
                &mut policy.spl_token.allow_unwrap_lamports
            }
        }
    }

    // `actor` goes in the slot the validator reads for this role: owner for the account
    // operations, mint authority for mint-to, mint or freeze authority for initialize-mint,
    // freeze authority for freeze and thaw, current or new authority for set-authority, and a
    // signer for the multisig.
    fn instruction(self, actor: &Pubkey) -> Instruction {
        let program = spl_token_id();
        let account = Pubkey::new_unique();
        let mint = Pubkey::new_unique();
        let other = Pubkey::new_unique();
        let multisig = Pubkey::new_unique();
        match self {
            Self::Transfer => transfer(&program, &account, &other, actor, &[], 1).unwrap(),
            Self::TransferChecked => {
                transfer_checked(&program, &account, &mint, &other, actor, &[], 1, 0).unwrap()
            }
            Self::TransferMultisigSigner => {
                transfer(&program, &account, &other, &multisig, &[actor], 1).unwrap()
            }
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
            Self::SetAuthorityCurrent => set_authority(
                &program,
                &account,
                Some(&other),
                AuthorityType::AccountOwner,
                actor,
                &[],
            )
            .unwrap(),
            Self::SetAuthorityNew => set_authority(
                &program,
                &mint,
                Some(actor),
                AuthorityType::FreezeAccount,
                &other,
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
            Self::InitializeMintAuthority => {
                initialize_mint(&program, &mint, actor, None, 0).unwrap()
            }
            Self::InitializeMintFreezeAuthority => {
                initialize_mint(&program, &mint, &other, Some(actor), 0).unwrap()
            }
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
            // Gates the fee payer appearing among the multisig signers, not as an owner.
            Self::InitializeMultisig => {
                initialize_multisig(&program, &account, &[actor, &other], 1).unwrap()
            }
            Self::FreezeAccount => freeze_account(&program, &account, &mint, actor, &[]).unwrap(),
            Self::FreezeAccountMultisigSigner => {
                freeze_account(&program, &account, &mint, &multisig, &[actor]).unwrap()
            }
            Self::ThawAccount => thaw_account(&program, &account, &mint, actor, &[]).unwrap(),
            Self::ThawAccountMultisigSigner => {
                thaw_account(&program, &account, &mint, &multisig, &[actor]).unwrap()
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
        }
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(512))]

    #[test]
    fn fee_payer_role_gated_iff_flag_off(
        (role_idx, actor_is_fee_payer, flags) in role_and_flags::<SplTokenRole>(),
    ) {
        assert_role_gated_iff_flag_off::<SplTokenRole>(role_idx, actor_is_fee_payer, &flags)?;
    }
}
