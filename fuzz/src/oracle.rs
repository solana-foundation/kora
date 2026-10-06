use kora_lib::config::FeePayerPolicy;

use crate::scenario::{
    AltOp, Key, LoaderV3Op, LoaderV4Op, Op, Signers, SystemOp, Token2022Op, TokenOp, TokenProgram,
};

fn gated(used: bool, allowed: bool, rule: &'static str) -> Option<&'static str> {
    (used && !allowed).then_some(rule)
}

fn always(used: bool, rule: &'static str) -> Option<&'static str> {
    used.then_some(rule)
}

fn signs(authority: &Key, signers: &Signers) -> bool {
    authority.is_fee_payer() || signers.contains_fee_payer()
}

fn is_fee_payer(key: &Option<Key>) -> bool {
    key.is_some_and(Key::is_fee_payer)
}

pub fn expected_rejection(
    op: &Op,
    policy: &FeePayerPolicy,
    allow_durable_transactions: bool,
) -> Option<&'static str> {
    match op {
        Op::System(op) => system(op, policy, allow_durable_transactions),
        Op::Token(program, op) => token(*program, op, policy),
        Op::Token2022(op) => token_2022(op, policy),
        Op::SplBatch(ops) => {
            Op::batch_ops(ops).iter().find_map(|op| token(TokenProgram::Spl, op, policy))
        }
        Op::CreateAta { funder, .. } => {
            gated(funder.is_fee_payer(), policy.system.allow_create_account, "ATA create funder")
        }
        Op::Alt(op) => alt(op, policy),
        Op::LoaderV3(op) => loader_v3(op, policy),
        Op::LoaderV4(op) => loader_v4(op, policy),
        Op::Raw { .. } => None,
    }
}

fn system(
    op: &SystemOp,
    policy: &FeePayerPolicy,
    allow_durable_transactions: bool,
) -> Option<&'static str> {
    let system = &policy.system;
    match op {
        SystemOp::Transfer { from, .. } => {
            gated(from.is_fee_payer(), system.allow_transfer, "System Transfer")
        }
        SystemOp::TransferWithSeed { base, .. } => {
            gated(base.is_fee_payer(), system.allow_transfer, "System TransferWithSeed")
        }
        SystemOp::CreateAccount { from, to, .. } => gated(
            from.is_fee_payer() || to.is_fee_payer(),
            system.allow_create_account,
            "System CreateAccount",
        ),
        SystemOp::CreateAccountWithSeed { from, base, .. } => gated(
            from.is_fee_payer() || base.is_fee_payer(),
            system.allow_create_account,
            "System CreateAccountWithSeed",
        ),
        SystemOp::CreateAccountAllowPrefund { new_account, funder, lamports, .. } => gated(
            new_account.is_fee_payer() || (*lamports > 0 && is_fee_payer(funder)),
            system.allow_create_account,
            "System CreateAccountAllowPrefund",
        ),
        SystemOp::Assign { account, .. } => {
            gated(account.is_fee_payer(), system.allow_assign, "System Assign")
        }
        SystemOp::AssignWithSeed { base, .. } => {
            gated(base.is_fee_payer(), system.allow_assign, "System AssignWithSeed")
        }
        SystemOp::Allocate { account, .. } => {
            gated(account.is_fee_payer(), system.allow_allocate, "System Allocate")
        }
        SystemOp::AllocateWithSeed { base, .. } => {
            gated(base.is_fee_payer(), system.allow_allocate, "System AllocateWithSeed")
        }
        SystemOp::InitializeNonce { authority, .. } => gated(
            authority.is_fee_payer(),
            system.nonce.allow_initialize,
            "System InitializeNonceAccount",
        ),
        SystemOp::AdvanceNonce { authority, .. } => {
            always(!allow_durable_transactions, "durable transactions disabled").or(gated(
                authority.is_fee_payer(),
                system.nonce.allow_advance,
                "System AdvanceNonceAccount",
            ))
        }
        SystemOp::WithdrawNonce { authority, .. } => gated(
            authority.is_fee_payer(),
            system.nonce.allow_withdraw,
            "System WithdrawNonceAccount",
        ),
        SystemOp::AuthorizeNonce { authority, .. } => gated(
            authority.is_fee_payer(),
            system.nonce.allow_authorize,
            "System AuthorizeNonceAccount",
        ),
        SystemOp::UpgradeNonce { .. } => None,
    }
}

macro_rules! token_flag {
    ($policy:expr, $program:expr, $flag:ident) => {
        match $program {
            TokenProgram::Spl => $policy.spl_token.$flag,
            TokenProgram::Token2022 => $policy.token_2022.$flag,
        }
    };
}

fn token(program: TokenProgram, op: &TokenOp, policy: &FeePayerPolicy) -> Option<&'static str> {
    match op {
        TokenOp::Transfer { owner, signers, .. }
        | TokenOp::TransferChecked { owner, signers, .. } => gated(
            signs(owner, signers),
            token_flag!(policy, program, allow_transfer),
            "Token Transfer",
        ),
        TokenOp::Approve { owner, signers, .. }
        | TokenOp::ApproveChecked { owner, signers, .. } => gated(
            signs(owner, signers),
            token_flag!(policy, program, allow_approve),
            "Token Approve",
        ),
        TokenOp::Revoke { owner, signers, .. } => {
            gated(signs(owner, signers), token_flag!(policy, program, allow_revoke), "Token Revoke")
        }
        TokenOp::SetAuthority { new_authority, owner, signers, .. } => gated(
            signs(owner, signers) || is_fee_payer(new_authority),
            token_flag!(policy, program, allow_set_authority),
            "Token SetAuthority",
        ),
        TokenOp::MintTo { authority, signers, .. }
        | TokenOp::MintToChecked { authority, signers, .. } => gated(
            signs(authority, signers),
            token_flag!(policy, program, allow_mint_to),
            "Token MintTo",
        ),
        TokenOp::Burn { owner, signers, .. } | TokenOp::BurnChecked { owner, signers, .. } => {
            gated(signs(owner, signers), token_flag!(policy, program, allow_burn), "Token Burn")
        }
        TokenOp::CloseAccount { owner, signers, .. } => gated(
            signs(owner, signers),
            token_flag!(policy, program, allow_close_account),
            "Token CloseAccount",
        ),
        TokenOp::FreezeAccount { authority, signers, .. } => gated(
            signs(authority, signers),
            token_flag!(policy, program, allow_freeze_account),
            "Token FreezeAccount",
        ),
        TokenOp::ThawAccount { authority, signers, .. } => gated(
            signs(authority, signers),
            token_flag!(policy, program, allow_thaw_account),
            "Token ThawAccount",
        ),
        TokenOp::InitializeMint { mint_authority, freeze_authority, .. } => gated(
            mint_authority.is_fee_payer() || is_fee_payer(freeze_authority),
            token_flag!(policy, program, allow_initialize_mint),
            "Token InitializeMint",
        ),
        TokenOp::InitializeAccount { owner, .. } => gated(
            owner.is_fee_payer(),
            token_flag!(policy, program, allow_initialize_account),
            "Token InitializeAccount",
        ),
        TokenOp::InitializeMultisig { signers, .. } => gated(
            signers.contains_fee_payer(),
            token_flag!(policy, program, allow_initialize_multisig),
            "Token InitializeMultisig",
        ),
        TokenOp::WithdrawExcessLamports { authority, signers, .. } => gated(
            signs(authority, signers),
            token_flag!(policy, program, allow_withdraw_excess_lamports),
            "Token WithdrawExcessLamports",
        ),
        TokenOp::UnwrapLamports { authority, signers, .. } => gated(
            signs(authority, signers),
            token_flag!(policy, program, allow_unwrap_lamports),
            "Token UnwrapLamports",
        ),
    }
}

fn token_2022(op: &Token2022Op, policy: &FeePayerPolicy) -> Option<&'static str> {
    let token_2022 = &policy.token_2022;
    match op {
        Token2022Op::Reallocate { payer, owner, signers, .. } => {
            always(payer.is_fee_payer() || signs(owner, signers), "Token2022 Reallocate")
        }
        Token2022Op::Pause { authority, signers, .. } => {
            gated(signs(authority, signers), token_2022.allow_freeze_account, "Token2022 Pause")
        }
        Token2022Op::Resume { authority, signers, .. } => {
            gated(signs(authority, signers), token_2022.allow_thaw_account, "Token2022 Resume")
        }
        Token2022Op::InitializePausable { authority, .. } => gated(
            authority.is_fee_payer(),
            token_2022.allow_initialize_extension_authority,
            "Token2022 InitializePausable authority",
        ),
        Token2022Op::InitializeMintCloseAuthority { close_authority, .. } => gated(
            is_fee_payer(close_authority),
            token_2022.allow_initialize_extension_authority,
            "Token2022 InitializeMintCloseAuthority",
        ),
        Token2022Op::InitializePermanentDelegate { delegate, .. } => gated(
            delegate.is_fee_payer(),
            token_2022.allow_initialize_extension_authority,
            "Token2022 InitializePermanentDelegate",
        ),
    }
}

fn alt(op: &AltOp, policy: &FeePayerPolicy) -> Option<&'static str> {
    let alt = &policy.alt;
    match op {
        AltOp::Create { authority, payer, .. } => gated(
            authority.is_fee_payer() || payer.is_fee_payer(),
            alt.allow_create,
            "ALT CreateLookupTable",
        ),
        AltOp::Extend { authority, payer, .. } => gated(
            authority.is_fee_payer() || is_fee_payer(payer),
            alt.allow_extend,
            "ALT ExtendLookupTable",
        ),
        AltOp::Freeze { authority, .. } => {
            gated(authority.is_fee_payer(), alt.allow_freeze, "ALT FreezeLookupTable")
        }
        AltOp::Deactivate { authority, .. } => {
            gated(authority.is_fee_payer(), alt.allow_deactivate, "ALT DeactivateLookupTable")
        }
        AltOp::Close { authority, .. } => {
            gated(authority.is_fee_payer(), alt.allow_close, "ALT CloseLookupTable")
        }
    }
}

fn loader_v3(op: &LoaderV3Op, policy: &FeePayerPolicy) -> Option<&'static str> {
    let v3 = &policy.bpf_loader_upgradeable;
    match op {
        LoaderV3Op::InitializeBuffer { authority, .. } => {
            gated(is_fee_payer(authority), v3.allow_initialize_buffer, "LoaderV3 InitializeBuffer")
        }
        LoaderV3Op::Write { authority, .. } => {
            gated(authority.is_fee_payer(), v3.allow_write, "LoaderV3 Write")
        }
        LoaderV3Op::DeployWithMaxDataLen { payer, authority, .. } => gated(
            payer.is_fee_payer() || authority.is_fee_payer(),
            v3.allow_deploy_with_max_data_len,
            "LoaderV3 DeployWithMaxDataLen",
        ),
        LoaderV3Op::Upgrade { authority, .. } => {
            gated(authority.is_fee_payer(), v3.allow_upgrade, "LoaderV3 Upgrade")
        }
        LoaderV3Op::SetBufferAuthority { current, new, .. } => gated(
            current.is_fee_payer() || new.is_fee_payer(),
            v3.allow_set_authority,
            "LoaderV3 SetAuthority (buffer)",
        ),
        LoaderV3Op::SetUpgradeAuthority { current, new, .. } => gated(
            current.is_fee_payer() || is_fee_payer(new),
            v3.allow_set_authority,
            "LoaderV3 SetAuthority (program)",
        ),
        LoaderV3Op::SetBufferAuthorityChecked { current, new, .. }
        | LoaderV3Op::SetUpgradeAuthorityChecked { current, new, .. } => gated(
            current.is_fee_payer() || new.is_fee_payer(),
            v3.allow_set_authority_checked,
            "LoaderV3 SetAuthorityChecked",
        ),
        LoaderV3Op::Close { recipient, authority, .. } => always(
            is_fee_payer(authority) && !recipient.is_fee_payer(),
            "LoaderV3 Close drain guard",
        )
        .or(gated(is_fee_payer(authority), v3.allow_close, "LoaderV3 Close")),
        LoaderV3Op::ExtendProgram { payer, .. } => {
            gated(is_fee_payer(payer), v3.allow_extend_program, "LoaderV3 ExtendProgram")
        }
        LoaderV3Op::ExtendProgramChecked { authority, payer, .. } => gated(
            authority.is_fee_payer() || is_fee_payer(payer),
            v3.allow_extend_program_checked,
            "LoaderV3 ExtendProgramChecked",
        ),
        LoaderV3Op::Migrate { authority, .. } => {
            gated(authority.is_fee_payer(), v3.allow_migrate, "LoaderV3 Migrate")
        }
    }
}

fn loader_v4(op: &LoaderV4Op, policy: &FeePayerPolicy) -> Option<&'static str> {
    let v4 = &policy.loader_v4;
    match op {
        LoaderV4Op::Write { authority, .. } => {
            gated(authority.is_fee_payer(), v4.allow_write, "LoaderV4 Write")
        }
        LoaderV4Op::Copy { authority, .. } => {
            gated(authority.is_fee_payer(), v4.allow_copy, "LoaderV4 Copy")
        }
        LoaderV4Op::SetProgramLength { authority, recipient, .. } => always(
            authority.is_fee_payer()
                && recipient.is_some_and(|recipient| !recipient.is_fee_payer()),
            "LoaderV4 SetProgramLength drain guard",
        )
        .or(gated(
            authority.is_fee_payer() || is_fee_payer(recipient),
            v4.allow_set_program_length,
            "LoaderV4 SetProgramLength",
        )),
        LoaderV4Op::Deploy { authority, .. } => {
            gated(authority.is_fee_payer(), v4.allow_deploy, "LoaderV4 Deploy")
        }
        LoaderV4Op::Retract { authority, .. } => {
            gated(authority.is_fee_payer(), v4.allow_retract, "LoaderV4 Retract")
        }
        LoaderV4Op::TransferAuthority { current, new, .. } => gated(
            current.is_fee_payer() || new.is_fee_payer(),
            v4.allow_transfer_authority,
            "LoaderV4 TransferAuthority",
        ),
        LoaderV4Op::Finalize { authority, .. } => {
            gated(authority.is_fee_payer(), v4.allow_finalize, "LoaderV4 Finalize")
        }
    }
}
