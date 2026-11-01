use std::{fs, path::Path};

use base64::{engine::general_purpose::STANDARD, Engine};
use kora_lib::constant::{BPF_LOADER_UPGRADEABLE_PROGRAM_ID, LOADER_V4_PROGRAM_ID};
use serde::Serialize;
use solana_address_lookup_table_interface::instruction::ProgramInstruction as AltInstruction;
use solana_loader_v3_interface::instruction::UpgradeableLoaderInstruction;
use solana_loader_v4_interface::instruction::LoaderV4Instruction;
use solana_sdk::{
    hash::Hash,
    instruction::{AccountMeta, Instruction},
    message::{v0, AddressLookupTableAccount, Message, VersionedMessage},
    pubkey::Pubkey,
    signature::Signature,
    transaction::VersionedTransaction,
};
use solana_system_interface::instruction::SystemInstruction;
use spl_token_2022_interface::extension::{pausable, transfer_fee, transfer_hook, ExtensionType};
use spl_token_metadata_interface::state::Field;

const SEED_PREFIX: &str = "seed_";
const ALLOW_PREFUND_TAG: u32 = 13;
const P_TOKEN_BATCH: u8 = 255;

fn key(n: u8) -> Pubkey {
    Pubkey::new_from_array([n; 32])
}

fn payer() -> Pubkey {
    key(1)
}

fn metas(n: u8) -> Vec<AccountMeta> {
    (1..=n).map(|i| AccountMeta::new(key(i), i == 1)).collect()
}

fn bincode_ix<T: Serialize>(program_id: Pubkey, ix: &T, accounts: u8) -> Instruction {
    Instruction::new_with_bytes(program_id, &bincode::serialize(ix).unwrap(), metas(accounts))
}

fn system(ix: SystemInstruction, accounts: u8) -> Instruction {
    bincode_ix(solana_system_interface::program::ID, &ix, accounts)
}

fn alt(ix: AltInstruction, accounts: u8) -> Instruction {
    bincode_ix(solana_address_lookup_table_interface::program::ID, &ix, accounts)
}

fn bpf_upgradeable(ix: UpgradeableLoaderInstruction, accounts: u8) -> Instruction {
    bincode_ix(BPF_LOADER_UPGRADEABLE_PROGRAM_ID, &ix, accounts)
}

fn loader_v4(ix: LoaderV4Instruction, accounts: u8) -> Instruction {
    bincode_ix(LOADER_V4_PROGRAM_ID, &ix, accounts)
}

fn spl_token_cases() -> Vec<(&'static str, Instruction)> {
    use spl_token_interface::instruction as t;
    let p = &spl_token_interface::ID;
    let (a, b, c, d) = (&key(1), &key(2), &key(3), &key(4));
    vec![
        ("spl_transfer", t::transfer(p, b, c, a, &[], 1_000).unwrap()),
        ("spl_transfer_checked", t::transfer_checked(p, b, d, c, a, &[], 1_000, 6).unwrap()),
        ("spl_transfer_multisig", t::transfer(p, b, c, a, &[d, &key(5)], 1).unwrap()),
        ("spl_burn", t::burn(p, b, d, a, &[], 5).unwrap()),
        ("spl_close_account", t::close_account(p, b, c, a, &[]).unwrap()),
        ("spl_approve", t::approve(p, b, c, a, &[], 7).unwrap()),
        ("spl_revoke", t::revoke(p, b, a, &[]).unwrap()),
        (
            "spl_set_authority",
            t::set_authority(p, b, Some(c), t::AuthorityType::AccountOwner, a, &[]).unwrap(),
        ),
        ("spl_mint_to", t::mint_to(p, d, b, a, &[], 9).unwrap()),
        ("spl_initialize_mint2", t::initialize_mint2(p, d, a, Some(a), 6).unwrap()),
        ("spl_initialize_account3", t::initialize_account3(p, b, d, a).unwrap()),
        ("spl_freeze_account", t::freeze_account(p, b, d, a, &[]).unwrap()),
    ]
}

fn p_token_batch() -> Instruction {
    use spl_token_interface::instruction as t;
    let p = &spl_token_interface::ID;
    let subs = [
        t::transfer(p, &key(2), &key(3), &key(1), &[], 10).unwrap(),
        t::close_account(p, &key(2), &key(1), &key(1), &[]).unwrap(),
    ];
    let mut data = vec![P_TOKEN_BATCH];
    let mut accounts = Vec::new();
    for sub in subs {
        data.push(sub.accounts.len() as u8);
        data.push(sub.data.len() as u8);
        data.extend_from_slice(&sub.data);
        accounts.extend(sub.accounts);
    }
    Instruction::new_with_bytes(*p, &data, accounts)
}

fn token_2022_cases() -> Vec<(&'static str, Instruction)> {
    use spl_token_2022_interface::instruction as t;
    let p = &spl_token_2022_interface::ID;
    let (a, b, c, d) = (&key(1), &key(2), &key(3), &key(4));
    vec![
        ("t22_transfer_checked", t::transfer_checked(p, b, d, c, a, &[], 1_000, 6).unwrap()),
        (
            "t22_transfer_checked_with_fee",
            transfer_fee::instruction::transfer_checked_with_fee(p, b, d, c, a, &[], 1_000, 6, 10)
                .unwrap(),
        ),
        (
            "t22_set_authority",
            t::set_authority(p, d, Some(c), t::AuthorityType::MintTokens, a, &[]).unwrap(),
        ),
        ("t22_initialize_mint2", t::initialize_mint2(p, d, a, None, 9).unwrap()),
        (
            "t22_initialize_mint_close_authority",
            t::initialize_mint_close_authority(p, d, Some(a)).unwrap(),
        ),
        ("t22_initialize_permanent_delegate", t::initialize_permanent_delegate(p, d, a).unwrap()),
        ("t22_reallocate", t::reallocate(p, b, a, a, &[], &[ExtensionType::MemoTransfer]).unwrap()),
        ("t22_pausable_initialize", pausable::instruction::initialize(p, d, a).unwrap()),
        ("t22_pausable_pause", pausable::instruction::pause(p, d, a, &[]).unwrap()),
        (
            "t22_transfer_hook_initialize",
            transfer_hook::instruction::initialize(p, d, Some(*a), Some(key(9))).unwrap(),
        ),
        (
            "t22_transfer_hook_update",
            transfer_hook::instruction::update(p, d, a, &[], None).unwrap(),
        ),
        (
            "t22_metadata_update_field",
            spl_token_metadata_interface::instruction::update_field(
                p,
                d,
                a,
                Field::Name,
                "seed".to_string(),
            ),
        ),
    ]
}

fn system_cases() -> Vec<(&'static str, Instruction)> {
    let owner = key(9);
    vec![
        ("system_transfer", system(SystemInstruction::Transfer { lamports: 5_000 }, 2)),
        (
            "system_create_account",
            system(SystemInstruction::CreateAccount { lamports: 1, space: 165, owner }, 2),
        ),
        (
            "system_create_account_with_seed",
            system(
                SystemInstruction::CreateAccountWithSeed {
                    base: key(3),
                    seed: "seed".to_string(),
                    lamports: 1,
                    space: 0,
                    owner,
                },
                3,
            ),
        ),
        (
            "system_create_account_allow_prefund",
            bincode_ix(
                solana_system_interface::program::ID,
                &(ALLOW_PREFUND_TAG, 1u64, 82u64, owner),
                2,
            ),
        ),
        (
            "system_transfer_with_seed",
            system(
                SystemInstruction::TransferWithSeed {
                    lamports: 1,
                    from_seed: "seed".to_string(),
                    from_owner: owner,
                },
                3,
            ),
        ),
        ("system_assign", system(SystemInstruction::Assign { owner }, 1)),
        ("system_allocate", system(SystemInstruction::Allocate { space: 64 }, 1)),
        (
            "system_allocate_with_seed",
            system(
                SystemInstruction::AllocateWithSeed {
                    base: key(1),
                    seed: "s".to_string(),
                    space: 8,
                    owner,
                },
                2,
            ),
        ),
        ("nonce_initialize", system(SystemInstruction::InitializeNonceAccount(key(1)), 3)),
        ("nonce_advance", system(SystemInstruction::AdvanceNonceAccount, 3)),
        ("nonce_withdraw", system(SystemInstruction::WithdrawNonceAccount(1_000), 5)),
        ("nonce_authorize", system(SystemInstruction::AuthorizeNonceAccount(key(7)), 2)),
        ("nonce_upgrade", system(SystemInstruction::UpgradeNonceAccount, 1)),
    ]
}

fn alt_cases() -> Vec<(&'static str, Instruction)> {
    vec![
        (
            "alt_create",
            alt(AltInstruction::CreateLookupTable { recent_slot: 42, bump_seed: 255 }, 4),
        ),
        (
            "alt_extend_with_payer",
            alt(AltInstruction::ExtendLookupTable { new_addresses: vec![key(8), key(9)] }, 4),
        ),
        ("alt_freeze", alt(AltInstruction::FreezeLookupTable, 2)),
        ("alt_deactivate", alt(AltInstruction::DeactivateLookupTable, 2)),
        ("alt_close", alt(AltInstruction::CloseLookupTable, 3)),
    ]
}

fn bpf_upgradeable_cases() -> Vec<(&'static str, Instruction)> {
    use UpgradeableLoaderInstruction as I;
    vec![
        ("bpf_initialize_buffer", bpf_upgradeable(I::InitializeBuffer, 2)),
        (
            "bpf_write",
            bpf_upgradeable(I::Write { offset: 0, bytes: vec![0x7f, b'E', b'L', b'F'] }, 2),
        ),
        ("bpf_deploy", bpf_upgradeable(I::DeployWithMaxDataLen { max_data_len: 1024 }, 8)),
        ("bpf_upgrade", bpf_upgradeable(I::Upgrade, 7)),
        ("bpf_set_authority", bpf_upgradeable(I::SetAuthority, 3)),
        ("bpf_set_authority_checked", bpf_upgradeable(I::SetAuthorityChecked, 3)),
        ("bpf_close", bpf_upgradeable(I::Close, 4)),
        ("bpf_extend_program", bpf_upgradeable(I::ExtendProgram { additional_bytes: 512 }, 4)),
        (
            "bpf_extend_program_checked",
            bpf_upgradeable(I::ExtendProgramChecked { additional_bytes: 512 }, 5),
        ),
        ("bpf_migrate", bpf_upgradeable(I::Migrate, 3)),
    ]
}

fn loader_v4_cases() -> Vec<(&'static str, Instruction)> {
    use LoaderV4Instruction as I;
    vec![
        ("v4_write", loader_v4(I::Write { offset: 0, bytes: vec![1, 2, 3, 4] }, 2)),
        ("v4_copy", loader_v4(I::Copy { destination_offset: 0, source_offset: 0, length: 4 }, 3)),
        ("v4_set_program_length", loader_v4(I::SetProgramLength { new_size: 4096 }, 3)),
        ("v4_deploy", loader_v4(I::Deploy, 2)),
        ("v4_retract", loader_v4(I::Retract, 2)),
        ("v4_transfer_authority", loader_v4(I::TransferAuthority, 3)),
        ("v4_finalize", loader_v4(I::Finalize, 3)),
    ]
}

fn sign_placeholder(message: VersionedMessage) -> VersionedTransaction {
    let signatures = vec![Signature::default(); message.header().num_required_signatures as usize];
    VersionedTransaction { signatures, message }
}

fn legacy(ixs: &[Instruction]) -> VersionedTransaction {
    let mut message = Message::new(ixs, Some(&payer()));
    message.recent_blockhash = Hash::new_from_array([7; 32]);
    sign_placeholder(VersionedMessage::Legacy(message))
}

fn v0(ixs: &[Instruction], tables: &[AddressLookupTableAccount]) -> VersionedTransaction {
    let message =
        v0::Message::try_compile(&payer(), ixs, tables, Hash::new_from_array([7; 32])).unwrap();
    sign_placeholder(VersionedMessage::V0(message))
}

fn transactions() -> Vec<(String, VersionedTransaction)> {
    let mut out = Vec::new();
    let groups = [
        system_cases(),
        spl_token_cases(),
        token_2022_cases(),
        alt_cases(),
        bpf_upgradeable_cases(),
        loader_v4_cases(),
    ];
    for (name, ix) in groups.iter().flatten() {
        out.push(((*name).to_string(), legacy(std::slice::from_ref(ix))));
    }

    out.push(("spl_p_token_batch".to_string(), legacy(&[p_token_batch()])));
    out.push(("legacy_empty".to_string(), legacy(&[])));

    let transfer = system(SystemInstruction::Transfer { lamports: 1 }, 2);
    let advance = system(SystemInstruction::AdvanceNonceAccount, 3);
    let spl = spl_token_cases().remove(0).1;
    out.push(("legacy_nonce_then_transfer".to_string(), legacy(&[advance, transfer.clone()])));
    out.push((
        "legacy_mixed_programs".to_string(),
        legacy(&[transfer.clone(), spl.clone(), alt(AltInstruction::FreezeLookupTable, 2)]),
    ));

    out.push(("v0_system_transfer".to_string(), v0(&[transfer], &[])));
    out.push(("v0_spl_transfer".to_string(), v0(std::slice::from_ref(&spl), &[])));
    out.push((
        "v0_t22_transfer_checked_with_fee".to_string(),
        v0(&[token_2022_cases().remove(1).1], &[]),
    ));
    out.push((
        "v0_loader_v4_deploy".to_string(),
        v0(&[loader_v4(LoaderV4Instruction::Deploy, 2)], &[]),
    ));
    let table = AddressLookupTableAccount { key: key(200), addresses: vec![key(3), key(4)] };
    out.push(("v0_with_lookup_table".to_string(), v0(&[spl], &[table])));
    out
}

fn reset_seeds(dir: &Path) {
    fs::create_dir_all(dir).unwrap();
    for entry in fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.file_name().unwrap().to_string_lossy().starts_with(SEED_PREFIX) {
            fs::remove_file(path).unwrap();
        }
    }
}

fn main() {
    let corpus = Path::new(env!("CARGO_MANIFEST_DIR")).join("corpus");
    let parse_dir = corpus.join("parse_transaction");
    let b64_dir = corpus.join("decode_b64_transaction");
    reset_seeds(&parse_dir);
    reset_seeds(&b64_dir);

    let b64_names = [
        "system_transfer",
        "nonce_advance",
        "spl_transfer_multisig",
        "t22_transfer_checked_with_fee",
        "legacy_empty",
        "legacy_mixed_programs",
        "v0_system_transfer",
        "v0_with_lookup_table",
    ];

    let txs = transactions();
    for (name, tx) in &txs {
        let bytes = wincode::serialize(tx).unwrap();
        fs::write(parse_dir.join(format!("{SEED_PREFIX}{name}")), &bytes).unwrap();
        if b64_names.contains(&name.as_str()) {
            fs::write(b64_dir.join(format!("{SEED_PREFIX}{name}")), STANDARD.encode(&bytes))
                .unwrap();
        }
    }
    println!(
        "wrote {} parse_transaction and {} decode_b64_transaction seeds",
        txs.len(),
        b64_names.len()
    );
}
