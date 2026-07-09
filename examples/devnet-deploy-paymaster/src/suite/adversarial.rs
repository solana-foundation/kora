use anyhow::Result;
use solana_compute_budget_interface::ComputeBudgetInstruction;
use solana_loader_v3_interface::{instruction as v3, state::UpgradeableLoaderState};
use solana_loader_v4_interface::instruction as v4;
use solana_sdk::{instruction::Instruction, pubkey::Pubkey, signature::Keypair, signer::Signer};
use solana_system_interface::{instruction as system_instruction, program as system_program};

use super::harness::{Harness, Probe, BPF_LOADER_UPGRADEABLE};

const LAMPORTS_PER_SOL: u64 = 1_000_000_000;
const CAP_LAMPORTS: u64 = 10 * LAMPORTS_PER_SOL;

pub struct Report {
    pub signed: u32,
    pub total: usize,
}

enum Kind {
    Theft,
    CapControl,
    Lock,
    Regression,
}

pub async fn run(h: &Harness, bytes: &[u8]) -> Result<Report> {
    let payer = h.payer();
    let attacker = Keypair::new();
    let attacker2 = Keypair::new();
    let owner = Keypair::new();

    println!("  attacker: {}", attacker.pubkey());
    println!("  provisioning real accounts ...");
    let probe_buffer = h.create_buffer().await?;
    let (program, program_data) = h.deploy_program(bytes, Some(&owner)).await?;
    let big_buffer = h.create_buffer_with_program(bytes).await?;
    let big_program = Keypair::new();
    let program_lamports = h
        .rpc()
        .get_minimum_balance_for_rent_exemption(UpgradeableLoaderState::size_of_program())
        .await?;
    let inflated_deploy = v3::deploy_with_max_program_len(
        &payer,
        &big_program.pubkey(),
        &big_buffer.pubkey(),
        &payer,
        program_lamports,
        1_700_000,
    )?;

    let mut rows: Vec<(&str, u8, Kind, Probe)> = Vec::new();
    macro_rules! case {
        ($name:expr, $tier:expr, $kind:expr, $ixs:expr, $signers:expr) => {
            rows.push(($name, $tier, $kind, h.probe($ixs, $signers).await?));
        };
    }

    case!(
        "create_account siphon to attacker (at cap)",
        1,
        Kind::Theft,
        &[create_account(&payer, &attacker.pubkey(), CAP_LAMPORTS)],
        &[&attacker]
    );
    case!(
        "create_account siphon to attacker (over cap)",
        1,
        Kind::CapControl,
        &[create_account(&payer, &attacker.pubkey(), CAP_LAMPORTS + 1)],
        &[&attacker]
    );
    case!(
        "priority-fee blowup",
        1,
        Kind::CapControl,
        &[
            ComputeBudgetInstruction::set_compute_unit_limit(1_400_000),
            ComputeBudgetInstruction::set_compute_unit_price(20_000_000_000),
            create_account(&payer, &attacker.pubkey(), 2_000_000),
        ],
        &[&attacker]
    );
    case!("deploy with inflated max_data_len", 2, Kind::Lock, &inflated_deploy, &[&big_program]);
    case!(
        "set_buffer_authority -> attacker",
        3,
        Kind::Regression,
        &[v3::set_buffer_authority(&probe_buffer.pubkey(), &payer, &attacker.pubkey())],
        &[]
    );
    case!(
        "close buffer -> attacker recipient",
        3,
        Kind::Regression,
        &[v3::close(&probe_buffer.pubkey(), &attacker.pubkey(), &payer)],
        &[]
    );
    case!(
        "extend_program_checked, attacker payer",
        3,
        Kind::Regression,
        &[v3::extend_program_checked(&program, &payer, Some(&attacker.pubkey()), 64)],
        &[&attacker]
    );
    case!(
        "migrate_program",
        3,
        Kind::Regression,
        &[v3::migrate_program(&program_data, &program, &payer)],
        &[]
    );
    case!(
        "system transfer kora -> attacker",
        3,
        Kind::Regression,
        &[system_instruction::transfer(&payer, &attacker.pubkey(), LAMPORTS_PER_SOL)],
        &[]
    );
    case!(
        "v4 finalize",
        3,
        Kind::Regression,
        &[v4::finalize(&program, &payer, &Pubkey::new_unique())],
        &[]
    );
    case!(
        "upgrade spill -> attacker",
        1,
        Kind::Theft,
        &[v3::upgrade(&program, &big_buffer.pubkey(), &payer, &attacker.pubkey())],
        &[]
    );
    case!(
        "upgrade without registered owner signature",
        1,
        Kind::Theft,
        &[v3::upgrade(&program, &big_buffer.pubkey(), &payer, &payer)],
        &[]
    );
    case!(
        "upgrade signed by non-owner wallet",
        1,
        Kind::Theft,
        &[with_extra_signer(
            v3::upgrade(&program, &big_buffer.pubkey(), &payer, &payer),
            &attacker.pubkey()
        )],
        &[&attacker]
    );
    case!(
        "close programdata without owner signature",
        1,
        Kind::Theft,
        &[v3::close_any(&program_data, &payer, Some(&payer), Some(&program))],
        &[]
    );
    case!(
        "allocate fee-payer account",
        3,
        Kind::Regression,
        &[system_instruction::allocate(&payer, 1024)],
        &[]
    );
    case!(
        "aggregate create_account > cap",
        1,
        Kind::CapControl,
        &[
            create_account_owned(&payer, &attacker.pubkey(), 6 * LAMPORTS_PER_SOL),
            create_account_owned(&payer, &attacker2.pubkey(), 6 * LAMPORTS_PER_SOL),
        ],
        &[&attacker, &attacker2]
    );

    let mut signed = 0u32;
    let mut last_tier = 0u8;
    for (name, tier, kind, probe) in &rows {
        if *tier != last_tier {
            println!("  ── tier {tier} ──");
            last_tier = *tier;
        }
        let verdict = match (kind, probe) {
            (Kind::Theft, Probe::Signed) => "⚠️  SIGNED — THEFT",
            (Kind::Lock, Probe::Signed) => "⚠️  SIGNED — capital lock",
            (_, Probe::Signed) => "❌ SIGNED",
            (_, Probe::PolicyReject(_)) => "✓ policy reject",
            (_, Probe::SimFailed(_)) => "·  not reachable (sim)",
        };
        if matches!(probe, Probe::Signed) {
            signed += 1;
        }
        println!("  {name:<46} {verdict}");
        if let Probe::PolicyReject(m) | Probe::SimFailed(m) = probe {
            println!("      {}", m.lines().next().unwrap_or(m));
        }
    }

    println!("  cleaning up ...");
    h.close_account(&probe_buffer.pubkey(), None, &[]).await.ok();
    h.close_account(&program_data, Some(&program), &[&owner]).await.ok();
    h.close_account(&big_buffer.pubkey(), None, &[]).await.ok();

    Ok(Report { signed, total: rows.len() })
}

fn with_extra_signer(mut ix: Instruction, signer: &Pubkey) -> Instruction {
    ix.accounts.push(solana_sdk::instruction::AccountMeta::new_readonly(*signer, true));
    ix
}

fn create_account(from: &Pubkey, to: &Pubkey, lamports: u64) -> Instruction {
    system_instruction::create_account(from, to, lamports, 0, &system_program::id())
}

fn create_account_owned(from: &Pubkey, to: &Pubkey, lamports: u64) -> Instruction {
    system_instruction::create_account(from, to, lamports, 0, &BPF_LOADER_UPGRADEABLE)
}
