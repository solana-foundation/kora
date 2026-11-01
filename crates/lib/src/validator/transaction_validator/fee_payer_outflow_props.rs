use super::*;
use crate::{
    oracle::utils::WSOL_DEVNET_MINT,
    tests::{
        account_mock::MintAccountMockBuilder, config_mock::ConfigMockBuilder,
        rpc_mock::RpcMockBuilder,
    },
    transaction::TransactionUtil,
};
use proptest::prelude::*;
use solana_address_lookup_table_interface::instruction::close_lookup_table;
use solana_client::rpc_request::RpcRequest;
use solana_message::Message;
use solana_sdk::instruction::Instruction;
use solana_system_interface::{
    instruction::{create_account, transfer, withdraw_nonce_account},
    program::ID as SYSTEM_PROGRAM_ID,
};
use spl_associated_token_account_interface::instruction::create_associated_token_account_idempotent;
use std::{collections::HashSet, str::FromStr};

const ATA_OWNER_POOL: u8 = 3;
const WSOL_DECIMALS: u32 = 9;

#[derive(Debug, Clone)]
enum Op {
    Transfer { from_fee_payer: bool, to_fee_payer: bool, lamports: u64 },
    CreateAccount { fee_payer_funds: bool, lamports: u64 },
    WithdrawNonce { fee_payer_authority: bool, fee_payer_recipient: bool, lamports: u64 },
    AltClose { fee_payer_authority: bool, fee_payer_recipient: bool },
    SplTransfer { fee_payer_owner: bool, amount: u64 },
    AtaCreate { fee_payer_funds: bool, owner_idx: u8 },
}

#[derive(Debug, Clone)]
struct Scenario {
    ops: Vec<Op>,
    ata_rent: u64,
    closed_account_lamports: u64,
    mint_decimals: u8,
    max_allowed_lamports: u64,
}

fn lamports() -> impl Strategy<Value = u64> {
    prop_oneof![
        3 => 0..2_000_000u64,
        2 => any::<u64>(),
        2 => (u64::MAX - 1_000)..=u64::MAX,
        1 => Just(u64::MAX),
        1 => Just((i64::MAX as u64) + 1),
    ]
}

fn op() -> impl Strategy<Value = Op> {
    prop_oneof![
        (any::<bool>(), any::<bool>(), lamports()).prop_map(
            |(from_fee_payer, to_fee_payer, lamports)| Op::Transfer {
                from_fee_payer,
                to_fee_payer,
                lamports
            }
        ),
        (any::<bool>(), lamports()).prop_map(|(fee_payer_funds, lamports)| Op::CreateAccount {
            fee_payer_funds,
            lamports
        }),
        (any::<bool>(), any::<bool>(), lamports()).prop_map(
            |(fee_payer_authority, fee_payer_recipient, lamports)| Op::WithdrawNonce {
                fee_payer_authority,
                fee_payer_recipient,
                lamports
            }
        ),
        (any::<bool>(), any::<bool>()).prop_map(|(fee_payer_authority, fee_payer_recipient)| {
            Op::AltClose { fee_payer_authority, fee_payer_recipient }
        }),
        (any::<bool>(), lamports())
            .prop_map(|(fee_payer_owner, amount)| Op::SplTransfer { fee_payer_owner, amount }),
        (any::<bool>(), 0..ATA_OWNER_POOL)
            .prop_map(|(fee_payer_funds, owner_idx)| Op::AtaCreate { fee_payer_funds, owner_idx }),
    ]
}

fn scenario() -> impl Strategy<Value = Scenario> {
    (
        prop::collection::vec(op(), 1..8),
        lamports(),
        lamports(),
        prop_oneof![Just(WSOL_DECIMALS as u8), 0..WSOL_DECIMALS as u8],
        lamports(),
        prop::option::of(-1i128..=1),
    )
        .prop_map(
            |(ops, ata_rent, closed_account_lamports, mint_decimals, cap, cap_offset_from_net)| {
                let mut scenario = Scenario {
                    ops,
                    ata_rent,
                    closed_account_lamports,
                    mint_decimals,
                    max_allowed_lamports: cap,
                };
                if let (Some(offset), Some(net)) =
                    (cap_offset_from_net, expected_net_outflow(&scenario))
                {
                    scenario.max_allowed_lamports =
                        (net + offset).clamp(0, u64::MAX as i128) as u64;
                }
                scenario
            },
        )
}

fn pick(is_fee_payer: bool, fee_payer: &Pubkey) -> Pubkey {
    if is_fee_payer {
        *fee_payer
    } else {
        Pubkey::new_unique()
    }
}

fn instructions(ops: &[Op], fee_payer: &Pubkey, ata_owners: &[Pubkey]) -> Vec<Instruction> {
    let wsol = Pubkey::from_str(WSOL_DEVNET_MINT).unwrap();
    let ata_mint = Pubkey::new_unique();
    ops.iter()
        .map(|op| match *op {
            Op::Transfer { from_fee_payer, to_fee_payer, lamports } => {
                transfer(&pick(from_fee_payer, fee_payer), &pick(to_fee_payer, fee_payer), lamports)
            }
            Op::CreateAccount { fee_payer_funds, lamports } => create_account(
                &pick(fee_payer_funds, fee_payer),
                &Pubkey::new_unique(),
                lamports,
                0,
                &SYSTEM_PROGRAM_ID,
            ),
            Op::WithdrawNonce { fee_payer_authority, fee_payer_recipient, lamports } => {
                withdraw_nonce_account(
                    &Pubkey::new_unique(),
                    &pick(fee_payer_authority, fee_payer),
                    &pick(fee_payer_recipient, fee_payer),
                    lamports,
                )
            }
            Op::AltClose { fee_payer_authority, fee_payer_recipient } => close_lookup_table(
                Pubkey::new_unique(),
                pick(fee_payer_authority, fee_payer),
                pick(fee_payer_recipient, fee_payer),
            ),
            Op::SplTransfer { fee_payer_owner, amount } => {
                spl_token_interface::instruction::transfer_checked(
                    &spl_token_interface::id(),
                    &Pubkey::new_unique(),
                    &wsol,
                    &Pubkey::new_unique(),
                    &pick(fee_payer_owner, fee_payer),
                    &[],
                    amount,
                    0,
                )
                .unwrap()
            }
            Op::AtaCreate { fee_payer_funds, owner_idx } => {
                create_associated_token_account_idempotent(
                    &pick(fee_payer_funds, fee_payer),
                    &ata_owners[owner_idx as usize],
                    &ata_mint,
                    &spl_token_interface::id(),
                )
            }
        })
        .collect()
}

fn expected_net_outflow(scenario: &Scenario) -> Option<i128> {
    let mut net: i128 = 0;
    let mut funded_atas = HashSet::new();
    let mut spl_total: u128 = 0;
    let spl_scale = 10u128.pow(WSOL_DECIMALS - scenario.mint_decimals as u32);
    let closed = scenario.closed_account_lamports as i128;

    for op in &scenario.ops {
        match *op {
            Op::Transfer { from_fee_payer, to_fee_payer, lamports } => {
                if from_fee_payer {
                    net += lamports as i128;
                }
                if to_fee_payer {
                    net -= lamports as i128;
                }
            }
            Op::CreateAccount { fee_payer_funds: true, lamports } => net += lamports as i128,
            Op::CreateAccount { fee_payer_funds: false, .. } => {}
            Op::WithdrawNonce { fee_payer_authority, fee_payer_recipient, lamports } => {
                match (fee_payer_authority, fee_payer_recipient) {
                    (true, false) => net += lamports as i128,
                    (false, true) => net -= lamports as i128,
                    _ => {}
                }
            }
            Op::AltClose { fee_payer_authority, fee_payer_recipient } => {
                match (fee_payer_authority, fee_payer_recipient) {
                    (true, false) => net += closed,
                    (false, true) => net -= closed,
                    _ => {}
                }
            }
            Op::SplTransfer { fee_payer_owner: true, amount } => {
                let value = amount as u128 * spl_scale;
                if value > u64::MAX as u128 {
                    return None;
                }
                spl_total += value;
            }
            Op::SplTransfer { fee_payer_owner: false, .. } => {}
            Op::AtaCreate { fee_payer_funds: true, owner_idx } => {
                funded_atas.insert(owner_idx);
            }
            Op::AtaCreate { fee_payer_funds: false, .. } => {}
        }
    }

    let ata_total = funded_atas.len() as u128 * scenario.ata_rent as u128;
    if ata_total > u64::MAX as u128 || spl_total > u64::MAX as u128 {
        return None;
    }
    Some(net + ata_total as i128 + spl_total as i128)
}

struct Harness {
    config: Config,
    fee_payer: Pubkey,
    resolved: VersionedTransactionResolved,
    rpc_client: std::sync::Arc<RpcClient>,
}

fn harness(scenario: &Scenario) -> Harness {
    let config = ConfigMockBuilder::new()
        .with_price_source(crate::oracle::PriceSource::Mock)
        .with_max_allowed_lamports(scenario.max_allowed_lamports)
        .build();
    let fee_payer = Pubkey::new_unique();
    let ata_owners: Vec<Pubkey> = (0..ATA_OWNER_POOL).map(|_| Pubkey::new_unique()).collect();
    let ixs = instructions(&scenario.ops, &fee_payer, &ata_owners);
    let message = VersionedMessage::Legacy(Message::new(&ixs, Some(&fee_payer)));
    let resolved = TransactionUtil::new_unsigned_versioned_transaction_resolved(message).unwrap();

    let mint_and_closed_account = MintAccountMockBuilder::new()
        .with_decimals(scenario.mint_decimals)
        .with_lamports(scenario.closed_account_lamports)
        .build();
    let rpc_client = RpcMockBuilder::new()
        .with_custom_mock(
            RpcRequest::GetMinimumBalanceForRentExemption,
            serde_json::json!(scenario.ata_rent),
        )
        .build_with_sequential_accounts(vec![&mint_and_closed_account; scenario.ops.len() + 1]);

    Harness { config, fee_payer, resolved, rpc_client }
}

fn block_on<F: std::future::Future>(future: F) -> F::Output {
    tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap().block_on(future)
}

#[test]
fn net_outflow_above_u64_is_rejected_not_truncated() {
    let scenario = Scenario {
        ops: vec![
            Op::Transfer { from_fee_payer: true, to_fee_payer: false, lamports: u64::MAX },
            Op::Transfer { from_fee_payer: true, to_fee_payer: false, lamports: 1 },
        ],
        ata_rent: 0,
        closed_account_lamports: 0,
        mint_decimals: 0,
        max_allowed_lamports: 0,
    };
    let Harness { config, fee_payer, mut resolved, rpc_client } = harness(&scenario);
    let validator = TransactionValidator::new(&config, fee_payer).unwrap();

    assert!(
        block_on(validator.validate_transfer_amounts(&config, &mut resolved, &rpc_client)).is_err()
    );
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(512))]

    #[test]
    fn outflow_matches_reference_and_never_panics(scenario in scenario()) {
        let Harness { config, fee_payer, mut resolved, rpc_client } = harness(&scenario);
        let actual = block_on(FeeConfigUtil::calculate_fee_payer_outflow(
            &fee_payer,
            &mut resolved,
            &rpc_client,
            &config,
        ));

        match expected_net_outflow(&scenario) {
            Some(expected) => prop_assert_eq!(actual.map_err(|e| e.to_string()), Ok(expected)),
            None => prop_assert!(actual.is_err(), "unrepresentable sub-total must error, got {actual:?}"),
        }
    }

    #[test]
    fn accepted_iff_net_outflow_within_cap(scenario in scenario()) {
        let Harness { config, fee_payer, mut resolved, rpc_client } = harness(&scenario);
        let validator = TransactionValidator::new(&config, fee_payer).unwrap();
        let accepted = block_on(validator.validate_transfer_amounts(
            &config,
            &mut resolved,
            &rpc_client,
        ))
        .is_ok();

        let within_cap = expected_net_outflow(&scenario)
            .is_some_and(|net| net <= scenario.max_allowed_lamports as i128);
        prop_assert_eq!(
            accepted,
            within_cap,
            "net outflow {:?} vs cap {}",
            expected_net_outflow(&scenario),
            scenario.max_allowed_lamports
        );
    }
}
