#![no_main]

use std::{cell::RefCell, collections::HashMap, sync::LazyLock};

use arbitrary::Arbitrary;
use async_trait::async_trait;
use base64::{engine::general_purpose::STANDARD, Engine};
use kora_fuzz::scenario::{Key, Scenario, TokenProgram, FEE_PAYER, POOL};
use kora_lib::{
    config::FeePayerPolicy, transaction::VersionedTransactionResolved,
    validator::transaction_validator::TransactionValidator,
};
use libfuzzer_sys::fuzz_target;
use litesvm::LiteSVM;
use serde_json::{json, Value};
use solana_client::{
    client_error::{ClientErrorKind, Result as ClientResult},
    nonblocking::rpc_client::RpcClient,
    rpc_client::RpcClientConfig,
    rpc_request::RpcRequest,
    rpc_sender::{RpcSender, RpcTransportStats},
};
use solana_message::{inner_instruction::InnerInstruction, VersionedMessage};
use solana_nonce::{
    state::{Data, DurableNonce, State},
    versions::Versions,
};
use solana_program_option::COption;
use solana_program_pack::Pack;
use solana_sdk::{
    account::{Account, ReadableAccount},
    hash::Hash,
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
};
use spl_token_2022_interface::{
    extension::StateWithExtensions,
    state::{Account as TokenAccount, AccountState, Mint},
};
use tokio::runtime::{Builder, Runtime};

const FEE_PAYER_LAMPORTS: u64 = 10_000_000_000;
const WALLET_LAMPORTS: u64 = 1_000_000_000;
const TOKEN_AMOUNT: u64 = 1_000_000_000;
const MINT_DECIMALS: u8 = 6;
const LAMPORTS_PER_SIGNATURE: u64 = 5_000;
const COMPUTE_UNIT_LIMIT: u32 = 1_400_000;
const LOADED_ACCOUNTS_DATA_SIZE_LIMIT: u32 = 64 * 1024 * 1024;
const FREEZE_ACCOUNT_TAG: u8 = 10;
const BATCH_TAG: u8 = 255;

static RUNTIME: LazyLock<Runtime> =
    LazyLock::new(|| Builder::new_current_thread().enable_all().build().unwrap());

thread_local! {
    static BASE_SVM: RefCell<LiteSVM> =
        RefCell::new(LiteSVM::new().with_sigverify(false).with_blockhash_check(false));
}

#[derive(Arbitrary, Debug, Clone, Copy)]
enum Seeded {
    Empty,
    Wallet,
    Mint { program: TokenProgram, authority: Key },
    TokenAccount { program: TokenProgram, mint: Key, owner: Key },
    Nonce { authority: Key },
}

#[derive(Arbitrary, Debug)]
struct Input {
    scenario: Scenario,
    max_allowed_lamports: u32,
    accounts: [Seeded; 3],
}

fn seeded_account(svm: &LiteSVM, seeded: Seeded) -> Option<Account> {
    let account = |owner: Pubkey, data: Vec<u8>| Account {
        lamports: svm.minimum_balance_for_rent_exemption(data.len()),
        data,
        owner,
        executable: false,
        rent_epoch: 0,
    };
    match seeded {
        Seeded::Empty => None,
        Seeded::Wallet => Some(Account {
            lamports: WALLET_LAMPORTS,
            ..account(solana_system_interface::program::ID, vec![])
        }),
        Seeded::Mint { program, authority } => {
            let mut data = vec![0; Mint::LEN];
            Mint {
                mint_authority: COption::Some(authority.pubkey()),
                supply: TOKEN_AMOUNT,
                decimals: MINT_DECIMALS,
                is_initialized: true,
                freeze_authority: COption::Some(authority.pubkey()),
            }
            .pack_into_slice(&mut data);
            Some(account(program.id(), data))
        }
        Seeded::TokenAccount { program, mint, owner } => {
            let mut data = vec![0; TokenAccount::LEN];
            TokenAccount {
                mint: mint.pubkey(),
                owner: owner.pubkey(),
                amount: TOKEN_AMOUNT,
                delegate: COption::None,
                state: AccountState::Initialized,
                is_native: COption::None,
                delegated_amount: 0,
                close_authority: COption::None,
            }
            .pack_into_slice(&mut data);
            Some(account(program.id(), data))
        }
        Seeded::Nonce { authority } => {
            let state = State::Initialized(Data::new(
                authority.pubkey(),
                DurableNonce::from_blockhash(&Hash::default()),
                LAMPORTS_PER_SIGNATURE,
            ));
            let data = wincode::serialize(&Versions::new(state)).ok()?;
            Some(account(solana_system_interface::program::ID, data))
        }
    }
}

struct Ledger {
    svm: LiteSVM,
    simulation: Value,
}

#[async_trait]
impl RpcSender for Ledger {
    async fn send(&self, request: RpcRequest, params: Value) -> ClientResult<Value> {
        match request {
            RpcRequest::GetAccountInfo => {
                let pubkey = params[0]
                    .as_str()
                    .and_then(|key| key.parse::<Pubkey>().ok())
                    .ok_or_else(|| ClientErrorKind::Custom("bad pubkey".to_string()))?;
                let value = self.svm.get_account(&pubkey).map(|account| {
                    json!({
                        "data": [STANDARD.encode(&account.data), "base64"],
                        "executable": account.executable,
                        "lamports": account.lamports,
                        "owner": account.owner.to_string(),
                        "rentEpoch": account.rent_epoch,
                        "space": account.data.len(),
                    })
                });
                Ok(json!({ "context": { "slot": 1 }, "value": value }))
            }
            RpcRequest::GetMinimumBalanceForRentExemption => {
                let len = params[0].as_u64().unwrap_or_default();
                Ok(json!(self.svm.minimum_balance_for_rent_exemption(len as usize)))
            }
            RpcRequest::SimulateTransaction => Ok(self.simulation.clone()),
            _ => Err(ClientErrorKind::Custom(format!("unmocked {request}")).into()),
        }
    }

    fn get_transport_stats(&self) -> RpcTransportStats {
        RpcTransportStats::default()
    }

    fn url(&self) -> String {
        "litesvm".to_string()
    }
}

fn simulation_response(inner: &[Vec<InnerInstruction>]) -> Value {
    let inner_instructions: Vec<Value> = inner
        .iter()
        .enumerate()
        .map(|(index, instructions)| {
            let instructions: Vec<Value> = instructions
                .iter()
                .map(|inner| {
                    json!({
                        "programIdIndex": inner.instruction.program_id_index,
                        "accounts": inner.instruction.accounts,
                        "data": bs58::encode(&inner.instruction.data).into_string(),
                        "stackHeight": inner.stack_height,
                    })
                })
                .collect();
            json!({ "index": index, "instructions": instructions })
        })
        .collect();
    json!({
        "context": { "slot": 1 },
        "value": { "err": null, "logs": [], "innerInstructions": inner_instructions },
    })
}

struct TokenFlags {
    transfer: bool,
    burn: bool,
    close_account: bool,
    approve: bool,
    set_authority: bool,
    mint_to: bool,
    freeze_account: bool,
}

fn token_flags(policy: &FeePayerPolicy, program: TokenProgram) -> TokenFlags {
    match program {
        TokenProgram::Spl => {
            let p = &policy.spl_token;
            TokenFlags {
                transfer: p.allow_transfer,
                burn: p.allow_burn,
                close_account: p.allow_close_account,
                approve: p.allow_approve,
                set_authority: p.allow_set_authority,
                mint_to: p.allow_mint_to,
                freeze_account: p.allow_freeze_account,
            }
        }
        TokenProgram::Token2022 => {
            let p = &policy.token_2022;
            TokenFlags {
                transfer: p.allow_transfer,
                burn: p.allow_burn,
                close_account: p.allow_close_account,
                approve: p.allow_approve,
                set_authority: p.allow_set_authority,
                mint_to: p.allow_mint_to,
                freeze_account: p.allow_freeze_account,
            }
        }
    }
}

fn unpack<S: Pack + spl_token_2022_interface::extension::BaseState>(
    program: TokenProgram,
    account: &Account,
) -> Option<S> {
    if account.owner != program.id() || account.lamports == 0 {
        return None;
    }
    StateWithExtensions::<S>::unpack(&account.data).ok().map(|state| state.base)
}

fn controlled(seeded: Seeded) -> bool {
    match seeded {
        Seeded::TokenAccount { owner, .. } => owner.is_fee_payer(),
        Seeded::Nonce { authority } => authority.is_fee_payer(),
        _ => false,
    }
}

fn authority_violation(
    seeded: Seeded,
    pre: &Account,
    post: &Account,
    policy: &FeePayerPolicy,
) -> Option<&'static str> {
    let deny =
        |changed: bool, allowed: bool, rule: &'static str| (changed && !allowed).then_some(rule);
    match seeded {
        Seeded::TokenAccount { program, owner, .. } if owner.is_fee_payer() => {
            let flags = token_flags(policy, program);
            let before: TokenAccount = unpack(program, pre)?;
            let Some(after) = unpack::<TokenAccount>(program, post) else {
                return deny(
                    true,
                    flags.close_account || flags.set_authority,
                    "fee payer token account closed",
                );
            };
            deny(
                after.owner != before.owner || after.close_authority != before.close_authority,
                flags.set_authority,
                "fee payer token account authority changed",
            )
            .or_else(|| {
                deny(
                    after.delegate.is_some(),
                    flags.approve || flags.set_authority,
                    "fee payer token account delegated",
                )
            })
            .or_else(|| {
                deny(
                    after.amount < before.amount,
                    flags.transfer || flags.burn || flags.approve || flags.set_authority,
                    "fee payer token account drained",
                )
            })
        }
        Seeded::Mint { program, authority } if authority.is_fee_payer() => {
            let flags = token_flags(policy, program);
            let before: Mint = unpack(program, pre)?;
            let after: Mint = unpack(program, post)?;
            deny(
                after.mint_authority != before.mint_authority
                    || after.freeze_authority != before.freeze_authority,
                flags.set_authority,
                "fee payer mint authority changed",
            )
            .or_else(|| {
                deny(
                    after.supply > before.supply,
                    flags.mint_to || flags.set_authority,
                    "fee payer mint supply inflated",
                )
            })
        }
        Seeded::Nonce { authority } if authority.is_fee_payer() => {
            let nonce = &policy.system.nonce;
            match wincode::deserialize::<Versions>(post.data()).ok().as_ref().map(Versions::state) {
                Some(State::Initialized(data)) => deny(
                    data.authority != FEE_PAYER,
                    nonce.allow_authorize,
                    "fee payer nonce authority changed",
                ),
                _ => deny(true, nonce.allow_withdraw, "fee payer nonce closed"),
            }
        }
        _ => None,
    }
}

fn signs_freeze(accounts: &[AccountMeta], data: &[u8], target: Pubkey) -> bool {
    match data.split_first() {
        Some((&FREEZE_ACCOUNT_TAG, _)) => {
            accounts.first().is_some_and(|meta| meta.pubkey == target)
                && accounts.iter().skip(2).any(|meta| meta.pubkey == FEE_PAYER)
        }
        Some((&BATCH_TAG, mut rest)) => {
            let mut accounts = accounts;
            while let [count, len, tail @ ..] = rest {
                let (count, len) = (usize::from(*count), usize::from(*len));
                if accounts.len() < count || tail.len() < len {
                    return false;
                }
                let (inner_accounts, next_accounts) = accounts.split_at(count);
                let (inner_data, next_data) = tail.split_at(len);
                if signs_freeze(inner_accounts, inner_data, target) {
                    return true;
                }
                accounts = next_accounts;
                rest = next_data;
            }
            false
        }
        _ => false,
    }
}

fn fee_payer_signed_freeze(instructions: &[Instruction], target: Pubkey) -> bool {
    instructions.iter().any(|ix| {
        (ix.program_id == spl_token_interface::ID || ix.program_id == spl_token_2022_interface::ID)
            && signs_freeze(&ix.accounts, &ix.data, target)
    })
}

fn freeze_violation(
    kind: Seeded,
    pubkey: Pubkey,
    pre: &Account,
    post: &Account,
    instructions: &[Instruction],
    policy: &FeePayerPolicy,
) -> Option<&'static str> {
    let Seeded::TokenAccount { program, .. } = kind else {
        return None;
    };
    let before: TokenAccount = unpack(program, pre)?;
    let after: TokenAccount = unpack(program, post)?;
    (before.state != AccountState::Frozen
        && after.state == AccountState::Frozen
        && fee_payer_signed_freeze(instructions, pubkey)
        && !token_flags(policy, program).freeze_account)
        .then_some("token account frozen by fee payer freeze authority")
}

fn fee_payer_violation(post: &Account, policy: &FeePayerPolicy) -> Option<&'static str> {
    let system = &policy.system;
    if post.owner != solana_system_interface::program::ID
        && !(system.allow_assign || system.allow_create_account)
    {
        return Some("fee payer reassigned");
    }
    if !post.data.is_empty() && !(system.allow_allocate || system.allow_create_account) {
        return Some("fee payer allocated");
    }
    None
}

fuzz_target!(|input: Input| {
    let scenario = &input.scenario;
    let instructions: Vec<Instruction> =
        scenario.built_ops().into_iter().map(|(_, ix)| ix).collect();
    if instructions.is_empty() {
        return;
    }
    let Some(mut transaction) = scenario.transaction(&instructions) else {
        return;
    };
    if let VersionedMessage::V1(message) = &mut transaction.message {
        message.config.compute_unit_limit = Some(COMPUTE_UNIT_LIMIT);
        message.config.loaded_accounts_data_size_limit = Some(LOADED_ACCOUNTS_DATA_SIZE_LIMIT);
    }

    let mut svm = BASE_SVM.with(|svm| svm.borrow().clone());
    let fee_payer =
        Account { lamports: FEE_PAYER_LAMPORTS, ..seeded_account(&svm, Seeded::Wallet).unwrap() };
    let mut seeded = vec![(FEE_PAYER, Seeded::Wallet, fee_payer)];
    for (pubkey, kind) in POOL[1..].iter().zip(input.accounts) {
        if let Some(account) = seeded_account(&svm, kind) {
            seeded.push((*pubkey, kind, account));
        }
    }
    for (pubkey, _, account) in &seeded {
        svm.set_account(*pubkey, account.clone()).unwrap();
    }

    let Ok(simulated) = svm.simulate_transaction(transaction.clone()) else {
        return;
    };
    let post: HashMap<Pubkey, Account> = simulated
        .post_accounts
        .into_iter()
        .map(|(pubkey, account)| (pubkey, account.into()))
        .collect();

    let config = scenario.config_with_max_allowed_lamports(input.max_allowed_lamports.into());
    let validator = TransactionValidator::new(&config, FEE_PAYER).unwrap();
    let rpc_client = RpcClient::new_sender(
        Ledger { svm, simulation: simulation_response(&simulated.meta.inner_instructions) },
        RpcClientConfig::default(),
    );
    let accepted = RUNTIME.block_on(async {
        let Ok(mut resolved) = VersionedTransactionResolved::from_transaction(
            &transaction,
            &config,
            &rpc_client,
            false,
            None,
        )
        .await
        else {
            return false;
        };
        validator.validate_transaction(&config, &mut resolved, &rpc_client).await.is_ok()
    });
    if !accepted {
        return;
    }

    let policy = &config.validation.fee_payer_policy;
    let mut pre_controlled = 0u128;
    let mut post_controlled = 0u128;
    for (pubkey, kind, pre) in &seeded {
        let after = post.get(pubkey).unwrap_or(pre);
        if *pubkey == FEE_PAYER {
            if let Some(rule) = fee_payer_violation(after, policy) {
                panic!("validator accepted a transaction where `{rule}`");
            }
        } else if let Some(rule) = authority_violation(*kind, pre, after, policy)
            .or_else(|| freeze_violation(*kind, *pubkey, pre, after, &instructions, policy))
        {
            panic!("validator accepted a transaction where `{rule}`");
        }
        if *pubkey == FEE_PAYER || controlled(*kind) {
            pre_controlled += u128::from(pre.lamports);
            post_controlled += u128::from(after.lamports);
        }
    }
    let loss = pre_controlled.saturating_sub(post_controlled);
    let budget = u128::from(simulated.meta.fee) + u128::from(input.max_allowed_lamports);
    assert!(
        loss <= budget,
        "fee payer lost {loss} lamports, more than fee {} + max_allowed_lamports {}",
        simulated.meta.fee,
        input.max_allowed_lamports
    );
});
