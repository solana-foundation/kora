use super::{assert_role_gated_iff_flag_off, role_and_flags, DrainRole};
use crate::config::FeePayerPolicy;
use proptest::prelude::*;
use solana_sdk::{instruction::Instruction, pubkey::Pubkey};
use spl_token_interface::{
    id as spl_token_id,
    instruction::{
        approve, burn, close_account, freeze_account, initialize_account, initialize_mint,
        initialize_multisig, mint_to, revoke, set_authority, thaw_account, transfer,
        unwrap_lamports, withdraw_excess_lamports, AuthorityType,
    },
};

#[derive(Debug, Clone, Copy)]
enum SplTokenRole {
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
    WithdrawExcessLamports,
    UnwrapLamports,
}

impl DrainRole for SplTokenRole {
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
        Self::WithdrawExcessLamports,
        Self::UnwrapLamports,
    ];

    fn allowed_programs() -> Vec<String> {
        vec![spl_token_id().to_string()]
    }

    fn flag(self, policy: &mut FeePayerPolicy) -> &mut bool {
        match self {
            Self::Transfer => &mut policy.spl_token.allow_transfer,
            Self::Burn => &mut policy.spl_token.allow_burn,
            Self::CloseAccount => &mut policy.spl_token.allow_close_account,
            Self::Approve => &mut policy.spl_token.allow_approve,
            Self::Revoke => &mut policy.spl_token.allow_revoke,
            Self::SetAuthority => &mut policy.spl_token.allow_set_authority,
            Self::MintTo => &mut policy.spl_token.allow_mint_to,
            Self::InitializeMint => &mut policy.spl_token.allow_initialize_mint,
            Self::InitializeAccount => &mut policy.spl_token.allow_initialize_account,
            Self::InitializeMultisig => &mut policy.spl_token.allow_initialize_multisig,
            Self::FreezeAccount => &mut policy.spl_token.allow_freeze_account,
            Self::ThawAccount => &mut policy.spl_token.allow_thaw_account,
            Self::WithdrawExcessLamports => &mut policy.spl_token.allow_withdraw_excess_lamports,
            Self::UnwrapLamports => &mut policy.spl_token.allow_unwrap_lamports,
        }
    }

    // `actor` goes in the slot the validator reads for this role: owner for the account
    // operations, mint authority for mint-to and initialize-mint, freeze authority for
    // freeze and thaw, current authority for set-authority, and a signer for the multisig.
    fn instruction(self, actor: &Pubkey) -> Instruction {
        let program = spl_token_id();
        let account = Pubkey::new_unique();
        let mint = Pubkey::new_unique();
        let other = Pubkey::new_unique();
        match self {
            Self::Transfer => transfer(&program, &account, &other, actor, &[], 1).unwrap(),
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
            // Gates the fee payer appearing among the multisig signers, not as an owner.
            Self::InitializeMultisig => {
                initialize_multisig(&program, &account, &[actor, &other], 1).unwrap()
            }
            Self::FreezeAccount => freeze_account(&program, &account, &mint, actor, &[]).unwrap(),
            Self::ThawAccount => thaw_account(&program, &account, &mint, actor, &[]).unwrap(),
            Self::WithdrawExcessLamports => {
                withdraw_excess_lamports(&program, &account, &other, actor, &[]).unwrap()
            }
            Self::UnwrapLamports => {
                unwrap_lamports(&program, &account, &other, actor, &[], None).unwrap()
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
