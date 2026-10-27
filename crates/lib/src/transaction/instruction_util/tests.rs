use super::*;
use crate::{
    constant::{
        instruction_indexes,
        instruction_indexes::system_create_account_allow_prefund::DISCRIMINATOR,
    },
    transaction::versioned_transaction::VersionedTransactionResolved,
};
use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    message::{AccountKeys, Message, VersionedMessage},
    signature::{Keypair, Signer},
    transaction::VersionedTransaction,
};
use solana_system_interface::{instruction::SystemInstruction, program::ID as SYSTEM_PROGRAM_ID};
use solana_transaction_status::parse_instruction;
use solana_transaction_status_client_types::{UiInstruction, UiParsedInstruction};

fn create_parsed_system_transfer(
    source: &Pubkey,
    destination: &Pubkey,
    lamports: u64,
) -> Result<solana_transaction_status_client_types::ParsedInstruction, Box<dyn std::error::Error>> {
    let solana_instruction =
        solana_system_interface::instruction::transfer(source, destination, lamports);

    let message = Message::new(&[solana_instruction], None);
    let compiled_instruction = &message.instructions[0];

    let account_keys_for_parsing = AccountKeys::new(&message.account_keys, None);

    let parsed = parse_instruction::parse(
        &SYSTEM_PROGRAM_ID,
        compiled_instruction,
        &account_keys_for_parsing,
        None,
    )?;

    Ok(parsed)
}

fn create_parsed_system_transfer_with_seed(
    source: &Pubkey,
    destination: &Pubkey,
    lamports: u64,
    source_base: &Pubkey,
    seed: &str,
    source_owner: &Pubkey,
) -> Result<solana_transaction_status_client_types::ParsedInstruction, Box<dyn std::error::Error>> {
    let solana_instruction = solana_system_interface::instruction::transfer_with_seed(
        source,
        source_base,
        seed.to_string(),
        source_owner,
        destination,
        lamports,
    );

    let message = Message::new(&[solana_instruction], None);
    let compiled_instruction = &message.instructions[0];

    let account_keys_for_parsing = AccountKeys::new(&message.account_keys, None);

    let parsed = parse_instruction::parse(
        &SYSTEM_PROGRAM_ID,
        compiled_instruction,
        &account_keys_for_parsing,
        None,
    )?;

    Ok(parsed)
}

fn create_parsed_system_create_account(
    source: &Pubkey,
    new_account: &Pubkey,
    lamports: u64,
    space: u64,
    owner: &Pubkey,
) -> Result<solana_transaction_status_client_types::ParsedInstruction, Box<dyn std::error::Error>> {
    let solana_instruction = solana_system_interface::instruction::create_account(
        source,
        new_account,
        lamports,
        space,
        owner,
    );

    let message = Message::new(&[solana_instruction], None);
    let compiled_instruction = &message.instructions[0];

    let account_keys_for_parsing = AccountKeys::new(&message.account_keys, None);

    let parsed = parse_instruction::parse(
        &SYSTEM_PROGRAM_ID,
        compiled_instruction,
        &account_keys_for_parsing,
        None,
    )?;

    Ok(parsed)
}

fn create_parsed_system_create_account_with_seed(
    source: &Pubkey,
    new_account: &Pubkey,
    base: &Pubkey,
    seed: &str,
    lamports: u64,
    space: u64,
    owner: &Pubkey,
) -> Result<solana_transaction_status_client_types::ParsedInstruction, Box<dyn std::error::Error>> {
    let solana_instruction = solana_system_interface::instruction::create_account_with_seed(
        source,
        new_account,
        base,
        seed,
        lamports,
        space,
        owner,
    );

    let message = Message::new(&[solana_instruction], None);
    let compiled_instruction = &message.instructions[0];

    let account_keys_for_parsing = AccountKeys::new(&message.account_keys, None);

    let parsed = parse_instruction::parse(
        &SYSTEM_PROGRAM_ID,
        compiled_instruction,
        &account_keys_for_parsing,
        None,
    )?;

    Ok(parsed)
}

fn create_parsed_system_assign(
    account: &Pubkey,
    owner: &Pubkey,
) -> Result<solana_transaction_status_client_types::ParsedInstruction, Box<dyn std::error::Error>> {
    let solana_instruction = solana_system_interface::instruction::assign(account, owner);

    let message = Message::new(&[solana_instruction], None);
    let compiled_instruction = &message.instructions[0];

    let account_keys_for_parsing = AccountKeys::new(&message.account_keys, None);

    let parsed = parse_instruction::parse(
        &SYSTEM_PROGRAM_ID,
        compiled_instruction,
        &account_keys_for_parsing,
        None,
    )?;

    Ok(parsed)
}

fn create_parsed_system_assign_with_seed(
    account: &Pubkey,
    base: &Pubkey,
    seed: &str,
    owner: &Pubkey,
) -> Result<solana_transaction_status_client_types::ParsedInstruction, Box<dyn std::error::Error>> {
    let solana_instruction =
        solana_system_interface::instruction::assign_with_seed(account, base, seed, owner);

    let message = Message::new(&[solana_instruction], None);
    let compiled_instruction = &message.instructions[0];

    let account_keys_for_parsing = AccountKeys::new(&message.account_keys, None);

    let parsed = parse_instruction::parse(
        &SYSTEM_PROGRAM_ID,
        compiled_instruction,
        &account_keys_for_parsing,
        None,
    )?;

    Ok(parsed)
}

fn create_parsed_system_withdraw_nonce_account(
    nonce_account: &Pubkey,
    nonce_authority: &Pubkey,
    recipient: &Pubkey,
    lamports: u64,
) -> Result<solana_transaction_status_client_types::ParsedInstruction, Box<dyn std::error::Error>> {
    let solana_instruction = solana_system_interface::instruction::withdraw_nonce_account(
        nonce_account,
        nonce_authority,
        recipient,
        lamports,
    );

    let message = Message::new(&[solana_instruction], None);
    let compiled_instruction = &message.instructions[0];

    let account_keys_for_parsing = AccountKeys::new(&message.account_keys, None);

    let parsed = parse_instruction::parse(
        &SYSTEM_PROGRAM_ID,
        compiled_instruction,
        &account_keys_for_parsing,
        None,
    )?;

    Ok(parsed)
}

fn create_parsed_spl_token_transfer(
    source: &Pubkey,
    destination: &Pubkey,
    authority: &Pubkey,
    amount: u64,
) -> Result<solana_transaction_status_client_types::ParsedInstruction, Box<dyn std::error::Error>> {
    let solana_instruction = spl_token_interface::instruction::transfer(
        &spl_token_interface::ID,
        source,
        destination,
        authority,
        &[],
        amount,
    )?;

    let message = Message::new(&[solana_instruction], None);
    let compiled_instruction = &message.instructions[0];

    let account_keys_for_parsing = AccountKeys::new(&message.account_keys, None);

    let parsed = parse_instruction::parse(
        &spl_token_interface::ID,
        compiled_instruction,
        &account_keys_for_parsing,
        None,
    )?;

    Ok(parsed)
}

fn create_parsed_spl_token_transfer_checked(
    source: &Pubkey,
    mint: &Pubkey,
    destination: &Pubkey,
    authority: &Pubkey,
    amount: u64,
    decimals: u8,
) -> Result<solana_transaction_status_client_types::ParsedInstruction, Box<dyn std::error::Error>> {
    let solana_instruction = spl_token_interface::instruction::transfer_checked(
        &spl_token_interface::ID,
        source,
        mint,
        destination,
        authority,
        &[],
        amount,
        decimals,
    )?;

    let message = Message::new(&[solana_instruction], None);
    let compiled_instruction = &message.instructions[0];

    let account_keys_for_parsing = AccountKeys::new(&message.account_keys, None);

    let parsed = parse_instruction::parse(
        &spl_token_interface::ID,
        compiled_instruction,
        &account_keys_for_parsing,
        None,
    )?;

    Ok(parsed)
}

fn create_parsed_spl_token_burn(
    account: &Pubkey,
    mint: &Pubkey,
    authority: &Pubkey,
    amount: u64,
) -> Result<solana_transaction_status_client_types::ParsedInstruction, Box<dyn std::error::Error>> {
    let solana_instruction = spl_token_interface::instruction::burn(
        &spl_token_interface::ID,
        account,
        mint,
        authority,
        &[],
        amount,
    )?;

    let message = Message::new(&[solana_instruction], None);
    let compiled_instruction = &message.instructions[0];

    let account_keys_for_parsing = AccountKeys::new(&message.account_keys, None);

    let parsed = parse_instruction::parse(
        &spl_token_interface::ID,
        compiled_instruction,
        &account_keys_for_parsing,
        None,
    )?;

    Ok(parsed)
}

fn create_parsed_spl_token_burn_checked(
    account: &Pubkey,
    mint: &Pubkey,
    authority: &Pubkey,
    amount: u64,
    decimals: u8,
) -> Result<solana_transaction_status_client_types::ParsedInstruction, Box<dyn std::error::Error>> {
    let solana_instruction = spl_token_interface::instruction::burn_checked(
        &spl_token_interface::ID,
        account,
        mint,
        authority,
        &[],
        amount,
        decimals,
    )?;

    let message = Message::new(&[solana_instruction], None);
    let compiled_instruction = &message.instructions[0];

    let account_keys_for_parsing = AccountKeys::new(&message.account_keys, None);

    let parsed = parse_instruction::parse(
        &spl_token_interface::ID,
        compiled_instruction,
        &account_keys_for_parsing,
        None,
    )?;

    Ok(parsed)
}

fn create_parsed_spl_token_close_account(
    account: &Pubkey,
    destination: &Pubkey,
    authority: &Pubkey,
) -> Result<solana_transaction_status_client_types::ParsedInstruction, Box<dyn std::error::Error>> {
    let solana_instruction = spl_token_interface::instruction::close_account(
        &spl_token_interface::ID,
        account,
        destination,
        authority,
        &[],
    )?;

    let message = Message::new(&[solana_instruction], None);
    let compiled_instruction = &message.instructions[0];

    let account_keys_for_parsing = AccountKeys::new(&message.account_keys, None);

    let parsed = parse_instruction::parse(
        &spl_token_interface::ID,
        compiled_instruction,
        &account_keys_for_parsing,
        None,
    )?;

    Ok(parsed)
}

fn create_parsed_spl_token_set_authority(
    owned: &Pubkey,
    authority_type: spl_token_interface::instruction::AuthorityType,
    new_authority: &Pubkey,
    authority: &Pubkey,
) -> Result<solana_transaction_status_client_types::ParsedInstruction, Box<dyn std::error::Error>> {
    let solana_instruction = spl_token_interface::instruction::set_authority(
        &spl_token_interface::ID,
        owned,
        Some(new_authority),
        authority_type,
        authority,
        &[],
    )?;

    let message = Message::new(&[solana_instruction], None);
    let compiled_instruction = &message.instructions[0];

    let account_keys_for_parsing = AccountKeys::new(&message.account_keys, None);

    let parsed = parse_instruction::parse(
        &spl_token_interface::ID,
        compiled_instruction,
        &account_keys_for_parsing,
        None,
    )?;

    Ok(parsed)
}

fn create_parsed_spl_token_sync_native(
    account: &Pubkey,
) -> Result<solana_transaction_status_client_types::ParsedInstruction, Box<dyn std::error::Error>> {
    let solana_instruction =
        spl_token_interface::instruction::sync_native(&spl_token_interface::ID, account)?;

    let message = Message::new(&[solana_instruction], None);
    let compiled_instruction = &message.instructions[0];

    let account_keys_for_parsing = AccountKeys::new(&message.account_keys, None);

    let parsed = parse_instruction::parse(
        &spl_token_interface::ID,
        compiled_instruction,
        &account_keys_for_parsing,
        None,
    )?;

    Ok(parsed)
}

fn create_parsed_spl_token_approve(
    source: &Pubkey,
    delegate: &Pubkey,
    authority: &Pubkey,
    amount: u64,
) -> Result<solana_transaction_status_client_types::ParsedInstruction, Box<dyn std::error::Error>> {
    let solana_instruction = spl_token_interface::instruction::approve(
        &spl_token_interface::ID,
        source,
        delegate,
        authority,
        &[],
        amount,
    )?;

    let message = Message::new(&[solana_instruction], None);
    let compiled_instruction = &message.instructions[0];

    let account_keys_for_parsing = AccountKeys::new(&message.account_keys, None);

    let parsed = parse_instruction::parse(
        &spl_token_interface::ID,
        compiled_instruction,
        &account_keys_for_parsing,
        None,
    )?;

    Ok(parsed)
}

fn create_parsed_spl_token_approve_checked(
    source: &Pubkey,
    mint: &Pubkey,
    delegate: &Pubkey,
    authority: &Pubkey,
    amount: u64,
    decimals: u8,
) -> Result<solana_transaction_status_client_types::ParsedInstruction, Box<dyn std::error::Error>> {
    let solana_instruction = spl_token_interface::instruction::approve_checked(
        &spl_token_interface::ID,
        source,
        mint,
        delegate,
        authority,
        &[],
        amount,
        decimals,
    )?;

    let message = Message::new(&[solana_instruction], None);
    let compiled_instruction = &message.instructions[0];

    let account_keys_for_parsing = AccountKeys::new(&message.account_keys, None);

    let parsed = parse_instruction::parse(
        &spl_token_interface::ID,
        compiled_instruction,
        &account_keys_for_parsing,
        None,
    )?;

    Ok(parsed)
}

fn create_parsed_token2022_transfer(
    source: &Pubkey,
    destination: &Pubkey,
    authority: &Pubkey,
    amount: u64,
) -> Result<solana_transaction_status_client_types::ParsedInstruction, Box<dyn std::error::Error>> {
    #[allow(deprecated)]
    let solana_instruction = spl_token_2022_interface::instruction::transfer(
        &spl_token_2022_interface::ID,
        source,
        destination,
        authority,
        &[],
        amount,
    )?;

    let message = Message::new(&[solana_instruction], None);
    let compiled_instruction = &message.instructions[0];

    let account_keys_for_parsing = AccountKeys::new(&message.account_keys, None);

    let parsed = parse_instruction::parse(
        &spl_token_2022_interface::ID,
        compiled_instruction,
        &account_keys_for_parsing,
        None,
    )?;

    Ok(parsed)
}

fn create_parsed_token2022_transfer_checked(
    source: &Pubkey,
    mint: &Pubkey,
    destination: &Pubkey,
    authority: &Pubkey,
    amount: u64,
    decimals: u8,
) -> Result<solana_transaction_status_client_types::ParsedInstruction, Box<dyn std::error::Error>> {
    let solana_instruction = spl_token_2022_interface::instruction::transfer_checked(
        &spl_token_2022_interface::ID,
        source,
        mint,
        destination,
        authority,
        &[],
        amount,
        decimals,
    )?;

    let message = Message::new(&[solana_instruction], None);
    let compiled_instruction = &message.instructions[0];

    let account_keys_for_parsing = AccountKeys::new(&message.account_keys, None);

    let parsed = parse_instruction::parse(
        &spl_token_2022_interface::ID,
        compiled_instruction,
        &account_keys_for_parsing,
        None,
    )?;

    Ok(parsed)
}

fn create_parsed_token2022_burn(
    account: &Pubkey,
    mint: &Pubkey,
    authority: &Pubkey,
    amount: u64,
) -> Result<solana_transaction_status_client_types::ParsedInstruction, Box<dyn std::error::Error>> {
    let solana_instruction = spl_token_2022_interface::instruction::burn(
        &spl_token_2022_interface::ID,
        account,
        mint,
        authority,
        &[],
        amount,
    )?;

    let message = Message::new(&[solana_instruction], None);
    let compiled_instruction = &message.instructions[0];

    let account_keys_for_parsing = AccountKeys::new(&message.account_keys, None);

    let parsed = parse_instruction::parse(
        &spl_token_2022_interface::ID,
        compiled_instruction,
        &account_keys_for_parsing,
        None,
    )?;

    Ok(parsed)
}

fn create_parsed_token2022_burn_checked(
    account: &Pubkey,
    mint: &Pubkey,
    authority: &Pubkey,
    amount: u64,
    decimals: u8,
) -> Result<solana_transaction_status_client_types::ParsedInstruction, Box<dyn std::error::Error>> {
    let solana_instruction = spl_token_2022_interface::instruction::burn_checked(
        &spl_token_2022_interface::ID,
        account,
        mint,
        authority,
        &[],
        amount,
        decimals,
    )?;

    let message = Message::new(&[solana_instruction], None);
    let compiled_instruction = &message.instructions[0];

    let account_keys_for_parsing = AccountKeys::new(&message.account_keys, None);

    let parsed = parse_instruction::parse(
        &spl_token_2022_interface::ID,
        compiled_instruction,
        &account_keys_for_parsing,
        None,
    )?;

    Ok(parsed)
}

fn create_parsed_token2022_close_account(
    account: &Pubkey,
    destination: &Pubkey,
    authority: &Pubkey,
) -> Result<solana_transaction_status_client_types::ParsedInstruction, Box<dyn std::error::Error>> {
    let solana_instruction = spl_token_2022_interface::instruction::close_account(
        &spl_token_2022_interface::ID,
        account,
        destination,
        authority,
        &[],
    )?;

    let message = Message::new(&[solana_instruction], None);
    let compiled_instruction = &message.instructions[0];

    let account_keys_for_parsing = AccountKeys::new(&message.account_keys, None);

    let parsed = parse_instruction::parse(
        &spl_token_2022_interface::ID,
        compiled_instruction,
        &account_keys_for_parsing,
        None,
    )?;

    Ok(parsed)
}

fn create_parsed_token2022_approve(
    source: &Pubkey,
    delegate: &Pubkey,
    authority: &Pubkey,
    amount: u64,
) -> Result<solana_transaction_status_client_types::ParsedInstruction, Box<dyn std::error::Error>> {
    let solana_instruction = spl_token_2022_interface::instruction::approve(
        &spl_token_2022_interface::ID,
        source,
        delegate,
        authority,
        &[],
        amount,
    )?;

    let message = Message::new(&[solana_instruction], None);
    let compiled_instruction = &message.instructions[0];

    let account_keys_for_parsing = AccountKeys::new(&message.account_keys, None);

    let parsed = parse_instruction::parse(
        &spl_token_2022_interface::ID,
        compiled_instruction,
        &account_keys_for_parsing,
        None,
    )?;

    Ok(parsed)
}

fn create_parsed_token2022_approve_checked(
    source: &Pubkey,
    mint: &Pubkey,
    delegate: &Pubkey,
    authority: &Pubkey,
    amount: u64,
    decimals: u8,
) -> Result<solana_transaction_status_client_types::ParsedInstruction, Box<dyn std::error::Error>> {
    let solana_instruction = spl_token_2022_interface::instruction::approve_checked(
        &spl_token_2022_interface::ID,
        source,
        mint,
        delegate,
        authority,
        &[],
        amount,
        decimals,
    )?;

    let message = Message::new(&[solana_instruction], None);
    let compiled_instruction = &message.instructions[0];

    let account_keys_for_parsing = AccountKeys::new(&message.account_keys, None);

    let parsed = parse_instruction::parse(
        &spl_token_2022_interface::ID,
        compiled_instruction,
        &account_keys_for_parsing,
        None,
    )?;

    Ok(parsed)
}

fn create_parsed_token2022_get_account_data_size(
    mint: &Pubkey,
    extension_types: &[spl_token_2022_interface::extension::ExtensionType],
) -> Result<solana_transaction_status_client_types::ParsedInstruction, Box<dyn std::error::Error>> {
    let solana_instruction = spl_token_2022_interface::instruction::get_account_data_size(
        &spl_token_2022_interface::ID,
        mint,
        extension_types,
    )?;

    let message = Message::new(&[solana_instruction], None);
    let compiled_instruction = &message.instructions[0];

    let account_keys_for_parsing = AccountKeys::new(&message.account_keys, None);

    let parsed = parse_instruction::parse(
        &spl_token_2022_interface::ID,
        compiled_instruction,
        &account_keys_for_parsing,
        None,
    )?;

    Ok(parsed)
}

#[test]
fn test_get_field_as_u64() {
    let valid_number = serde_json::json!({
        "amount": 1000
    });
    assert_eq!(IxUtils::get_field_as_u64(&valid_number, "amount").unwrap(), 1000);

    let valid_string_number = serde_json::json!({
        "amount": "2000"
    });
    assert_eq!(IxUtils::get_field_as_u64(&valid_string_number, "amount").unwrap(), 2000);

    let missing_field = serde_json::json!({
        "other": 3000
    });
    let err = IxUtils::get_field_as_u64(&missing_field, "amount").unwrap_err();
    assert!(matches!(err, crate::error::KoraError::SerializationError(_)));

    let invalid_string = serde_json::json!({
        "amount": "invalid"
    });
    let err = IxUtils::get_field_as_u64(&invalid_string, "amount").unwrap_err();
    assert!(matches!(err, crate::error::KoraError::SerializationError(_)));

    let null_value = serde_json::json!({
        "amount": null
    });
    let err = IxUtils::get_field_as_u64(&null_value, "amount").unwrap_err();
    assert!(matches!(err, crate::error::KoraError::SerializationError(_)));

    let array_value = serde_json::json!({
        "amount": [1, 2, 3]
    });
    let err = IxUtils::get_field_as_u64(&array_value, "amount").unwrap_err();
    assert!(matches!(err, crate::error::KoraError::SerializationError(_)));
}

#[test]
fn test_parse_system_instructions_transfer() {
    use crate::transaction::versioned_transaction::VersionedTransactionResolved;
    use solana_message::{Message, VersionedMessage};
    use solana_sdk::{
        signature::{Keypair, Signer},
        transaction::VersionedTransaction,
    };
    use solana_system_interface::instruction::transfer;

    let sender = Keypair::new();
    let receiver = Pubkey::new_unique();
    let lamports = 1000u64;

    let instruction = transfer(&sender.pubkey(), &receiver, lamports);

    let message = VersionedMessage::Legacy(Message::new(&[instruction], Some(&sender.pubkey())));
    let tx = VersionedTransaction::try_new(message, &[&sender]).unwrap();

    let resolved_tx = VersionedTransactionResolved::from_kora_built_transaction(&tx)
        .expect("Failed to create resolved transaction");

    let parsed_instructions = IxUtils::parse_system_instructions(&resolved_tx)
        .expect("Failed to parse system instructions");

    let transfers = parsed_instructions
        .get(&ParsedSystemInstructionType::SystemTransfer)
        .expect("Expected SystemTransfer instructions");

    assert_eq!(transfers.len(), 1);

    match &transfers[0] {
        ParsedSystemInstructionData::SystemTransfer {
            lamports: parsed_lamports,
            sender: parsed_sender,
            receiver: parsed_receiver,
        } => {
            assert_eq!(*parsed_lamports, lamports);
            assert_eq!(*parsed_sender, sender.pubkey());
            assert_eq!(*parsed_receiver, receiver);
        }
        _ => panic!("Expected SystemTransfer variant"),
    }
}

#[test]
fn test_parse_system_instructions_create_account() {
    use crate::transaction::versioned_transaction::VersionedTransactionResolved;
    use solana_message::{Message, VersionedMessage};
    use solana_sdk::{
        signature::{Keypair, Signer},
        transaction::VersionedTransaction,
    };
    use solana_system_interface::instruction::create_account;

    let payer = Keypair::new();
    let new_account = Keypair::new();
    let owner = Pubkey::new_unique();
    let lamports = 2_000_000u64;
    let space = 165u64;

    let instruction =
        create_account(&payer.pubkey(), &new_account.pubkey(), lamports, space, &owner);

    let message = VersionedMessage::Legacy(Message::new(&[instruction], Some(&payer.pubkey())));
    let tx = VersionedTransaction::try_new(message, &[&payer, &new_account]).unwrap();

    let resolved_tx = VersionedTransactionResolved::from_kora_built_transaction(&tx)
        .expect("Failed to create resolved transaction");

    let parsed_instructions = IxUtils::parse_system_instructions(&resolved_tx)
        .expect("Failed to parse system instructions");

    let creates = parsed_instructions
        .get(&ParsedSystemInstructionType::SystemCreateAccount)
        .expect("Expected SystemCreateAccount instructions");

    assert_eq!(creates.len(), 1);

    match &creates[0] {
        ParsedSystemInstructionData::SystemCreateAccount {
            lamports: parsed_lamports,
            payer: parsed_payer,
            new_account: parsed_new_account,
            owner: parsed_owner,
            base: parsed_base,
        } => {
            assert_eq!(*parsed_lamports, lamports);
            assert_eq!(*parsed_payer, payer.pubkey());
            assert_eq!(*parsed_new_account, new_account.pubkey());
            assert_eq!(*parsed_owner, owner);
            assert_eq!(*parsed_base, None);
        }
        _ => panic!("Expected SystemCreateAccount variant"),
    }
}

#[test]
fn test_parse_system_instructions_create_account_with_seed() {
    use crate::transaction::versioned_transaction::VersionedTransactionResolved;
    use solana_message::{Message, VersionedMessage};
    use solana_sdk::{
        signature::{Keypair, Signer},
        transaction::VersionedTransaction,
    };
    use solana_system_interface::instruction::create_account_with_seed;

    let payer = Keypair::new();
    let owner = Pubkey::new_unique();
    let seed = "seeded-account";
    let new_account = Pubkey::create_with_seed(&payer.pubkey(), seed, &owner).unwrap();
    let lamports = 3_000_000u64;
    let space = 200u64;

    let instruction = create_account_with_seed(
        &payer.pubkey(),
        &new_account,
        &payer.pubkey(),
        seed,
        lamports,
        space,
        &owner,
    );

    let message = VersionedMessage::Legacy(Message::new(&[instruction], Some(&payer.pubkey())));
    let tx = VersionedTransaction::try_new(message, &[&payer]).unwrap();

    let resolved_tx = VersionedTransactionResolved::from_kora_built_transaction(&tx)
        .expect("Failed to create resolved transaction");

    let parsed_instructions = IxUtils::parse_system_instructions(&resolved_tx)
        .expect("Failed to parse system instructions");

    let creates = parsed_instructions
        .get(&ParsedSystemInstructionType::SystemCreateAccount)
        .expect("Expected SystemCreateAccount instructions");

    assert_eq!(creates.len(), 1);

    match &creates[0] {
        ParsedSystemInstructionData::SystemCreateAccount {
            lamports: parsed_lamports,
            payer: parsed_payer,
            new_account: parsed_new_account,
            owner: parsed_owner,
            base: parsed_base,
        } => {
            assert_eq!(*parsed_lamports, lamports);
            assert_eq!(*parsed_payer, payer.pubkey());
            assert_eq!(*parsed_new_account, new_account);
            assert_eq!(*parsed_owner, owner);
            assert_eq!(*parsed_base, Some(payer.pubkey()));
        }
        _ => panic!("Expected SystemCreateAccount variant"),
    }
}

#[test]
fn test_create_account_allow_prefund_bincode_tag() {
    // Anchors the hand-coded tag 13: prefund follows UpgradeNonceAccount, so tag 12 here
    // proves it. Fails loud if upstream reorders the enum.
    let upgrade = bincode::serialize(&SystemInstruction::UpgradeNonceAccount).unwrap();
    assert_eq!(&upgrade[0..4], &12u32.to_le_bytes());
}

fn build_create_account_allow_prefund_ix(
    new_account: &Pubkey,
    funder: &Pubkey,
    lamports: u64,
    space: u64,
    owner: &Pubkey,
) -> Instruction {
    let mut accounts = vec![AccountMeta::new(*new_account, true)];
    if lamports > 0 {
        accounts.push(AccountMeta::new(*funder, true));
    }
    Instruction {
        program_id: SYSTEM_PROGRAM_ID,
        accounts,
        data: bincode::serialize(&(DISCRIMINATOR, lamports, space, *owner)).unwrap(),
    }
}

#[test]
fn test_parse_system_instructions_create_account_allow_prefund() {
    let funder = Keypair::new();
    let new_account = Keypair::new();
    let owner = Pubkey::new_unique();
    let lamports = 2_000_000u64;

    let instruction = build_create_account_allow_prefund_ix(
        &new_account.pubkey(),
        &funder.pubkey(),
        lamports,
        165,
        &owner,
    );

    let message = VersionedMessage::Legacy(Message::new(&[instruction], Some(&funder.pubkey())));
    let tx = VersionedTransaction::try_new(message, &[&funder, &new_account]).unwrap();

    let resolved_tx = VersionedTransactionResolved::from_kora_built_transaction(&tx)
        .expect("Failed to create resolved transaction");

    let parsed_instructions = IxUtils::parse_system_instructions(&resolved_tx)
        .expect("Failed to parse system instructions");

    let creates = parsed_instructions
        .get(&ParsedSystemInstructionType::SystemCreateAccount)
        .expect("Expected SystemCreateAccount instructions");

    assert_eq!(creates.len(), 1);
    match &creates[0] {
        ParsedSystemInstructionData::SystemCreateAccount {
            lamports: parsed_lamports,
            payer: parsed_payer,
            new_account: parsed_new_account,
            owner: parsed_owner,
            base: _,
        } => {
            assert_eq!(*parsed_lamports, lamports);
            assert_eq!(*parsed_payer, funder.pubkey());
            assert_eq!(*parsed_new_account, new_account.pubkey());
            assert_eq!(*parsed_owner, owner);
        }
        _ => panic!("Expected SystemCreateAccount variant"),
    }
}

#[test]
fn test_parse_system_instructions_create_account_allow_prefund_no_funding() {
    // lamports == 0 omits the funding account; the new account is the only signer and
    // is recorded as the payer so policy and outflow accounting treat it consistently.
    let new_account = Keypair::new();
    let owner = Pubkey::new_unique();

    let instruction = build_create_account_allow_prefund_ix(
        &new_account.pubkey(),
        &new_account.pubkey(),
        0,
        165,
        &owner,
    );

    let message =
        VersionedMessage::Legacy(Message::new(&[instruction], Some(&new_account.pubkey())));
    let tx = VersionedTransaction::try_new(message, &[&new_account]).unwrap();

    let resolved_tx = VersionedTransactionResolved::from_kora_built_transaction(&tx)
        .expect("Failed to create resolved transaction");

    let parsed_instructions = IxUtils::parse_system_instructions(&resolved_tx)
        .expect("Failed to parse system instructions");

    let creates = parsed_instructions
        .get(&ParsedSystemInstructionType::SystemCreateAccount)
        .expect("Expected SystemCreateAccount instructions");

    assert_eq!(creates.len(), 1);
    match &creates[0] {
        ParsedSystemInstructionData::SystemCreateAccount {
            lamports: parsed_lamports,
            payer: parsed_payer,
            new_account: parsed_new_account,
            ..
        } => {
            assert_eq!(*parsed_lamports, 0);
            assert_eq!(*parsed_payer, new_account.pubkey());
            assert_eq!(*parsed_new_account, new_account.pubkey());
        }
        _ => panic!("Expected SystemCreateAccount variant"),
    }
}

#[test]
fn test_parse_alt_instructions_freeze_lookup_table() {
    use crate::transaction::versioned_transaction::VersionedTransactionResolved;
    use solana_address_lookup_table_interface::instruction::freeze_lookup_table;
    use solana_message::{Message, VersionedMessage};
    use solana_sdk::{
        signature::{Keypair, Signer},
        transaction::VersionedTransaction,
    };

    let payer = Keypair::new();
    let lookup_table_account = Pubkey::new_unique();
    let authority = payer.pubkey();

    let instruction = freeze_lookup_table(lookup_table_account, authority);
    let message = VersionedMessage::Legacy(Message::new(&[instruction], Some(&payer.pubkey())));
    let tx = VersionedTransaction::try_new(message, &[&payer]).unwrap();

    let resolved_tx = VersionedTransactionResolved::from_kora_built_transaction(&tx)
        .expect("Failed to create resolved transaction");

    let parsed_instructions =
        IxUtils::parse_alt_instructions(&resolved_tx).expect("Failed to parse ALT instructions");

    let freezes = parsed_instructions
        .get(&ParsedALTInstructionType::AltFreezeLookupTable)
        .expect("Expected AltFreezeLookupTable instructions");

    assert_eq!(freezes.len(), 1);
    match &freezes[0] {
        ParsedALTInstructionData::AltFreezeLookupTable {
            lookup_table_account: parsed_lookup_table_account,
            lookup_table_authority: parsed_lookup_table_authority,
        } => {
            assert_eq!(*parsed_lookup_table_account, lookup_table_account);
            assert_eq!(*parsed_lookup_table_authority, authority);
        }
        _ => panic!("Expected AltFreezeLookupTable variant"),
    }
}

#[test]
fn test_parse_alt_instructions_extend_lookup_table_optional_payer() {
    use crate::transaction::versioned_transaction::VersionedTransactionResolved;
    use solana_address_lookup_table_interface::instruction::extend_lookup_table;
    use solana_message::{Message, VersionedMessage};
    use solana_sdk::{
        signature::{Keypair, Signer},
        transaction::VersionedTransaction,
    };

    let signer = Keypair::new();
    let lookup_table_account = Pubkey::new_unique();
    let authority = signer.pubkey();
    let payer = signer.pubkey();

    let instruction = extend_lookup_table(
        lookup_table_account,
        authority,
        Some(payer),
        vec![Pubkey::new_unique()],
    );
    let message = VersionedMessage::Legacy(Message::new(&[instruction], Some(&signer.pubkey())));
    let tx = VersionedTransaction::try_new(message, &[&signer]).unwrap();

    let resolved_tx = VersionedTransactionResolved::from_kora_built_transaction(&tx)
        .expect("Failed to create resolved transaction");

    let parsed_instructions =
        IxUtils::parse_alt_instructions(&resolved_tx).expect("Failed to parse ALT instructions");

    let extends = parsed_instructions
        .get(&ParsedALTInstructionType::AltExtendLookupTable)
        .expect("Expected AltExtendLookupTable instructions");

    assert_eq!(extends.len(), 1);
    match &extends[0] {
        ParsedALTInstructionData::AltExtendLookupTable {
            lookup_table_account: parsed_lookup_table_account,
            lookup_table_authority: parsed_lookup_table_authority,
            payer_account: parsed_payer_account,
        } => {
            assert_eq!(*parsed_lookup_table_account, lookup_table_account);
            assert_eq!(*parsed_lookup_table_authority, authority);
            assert_eq!(*parsed_payer_account, Some(payer));
        }
        _ => panic!("Expected AltExtendLookupTable variant"),
    }
}

#[test]
fn test_parse_token_instructions_transfer() {
    use crate::transaction::versioned_transaction::VersionedTransactionResolved;
    use solana_message::{Message, VersionedMessage};
    use solana_sdk::{
        signature::{Keypair, Signer},
        transaction::VersionedTransaction,
    };
    use spl_token_interface::instruction::transfer;

    let owner = Keypair::new();
    let source_address = Pubkey::new_unique();
    let destination_address = Pubkey::new_unique();
    let amount = 5000u64;

    let instruction = transfer(
        &spl_token_interface::ID,
        &source_address,
        &destination_address,
        &owner.pubkey(),
        &[],
        amount,
    )
    .expect("Failed to create transfer instruction");

    let message = VersionedMessage::Legacy(Message::new(&[instruction], Some(&owner.pubkey())));
    let tx = VersionedTransaction::try_new(message, &[&owner]).unwrap();

    let resolved_tx = VersionedTransactionResolved::from_kora_built_transaction(&tx)
        .expect("Failed to create resolved transaction");

    let parsed_instructions = IxUtils::parse_token_instructions(&resolved_tx)
        .expect("Failed to parse token instructions");

    let transfers = parsed_instructions
        .get(&ParsedSPLInstructionType::SplTokenTransfer)
        .expect("Expected SplTokenTransfer instructions");

    assert_eq!(transfers.len(), 1);

    match &transfers[0] {
        ParsedSPLInstructionData::SplTokenTransfer {
            amount: parsed_amount,
            owner: parsed_owner,
            multisig_signers,
            mint,
            source_address: parsed_source,
            destination_address: parsed_destination,
            is_2022,
        } => {
            assert_eq!(*parsed_amount, amount);
            assert_eq!(*parsed_owner, owner.pubkey());
            assert!(multisig_signers.is_empty());
            assert_eq!(*parsed_source, source_address);
            assert_eq!(*parsed_destination, destination_address);
            assert!(!is_2022);
            assert!(mint.is_none());
        }
        _ => panic!("Expected SplTokenTransfer variant"),
    }
}

#[test]
fn test_parse_token_2022_instructions_transfer_checked() {
    use crate::transaction::versioned_transaction::VersionedTransactionResolved;
    use solana_message::{Message, VersionedMessage};
    use solana_sdk::{
        signature::{Keypair, Signer},
        transaction::VersionedTransaction,
    };
    use spl_token_2022_interface::instruction::transfer_checked;

    let owner = Keypair::new();
    let source_address = Pubkey::new_unique();
    let mint = Pubkey::new_unique();
    let destination_address = Pubkey::new_unique();
    let amount = 7500u64;
    let decimals = 6u8;

    let instruction = transfer_checked(
        &spl_token_2022_interface::ID,
        &source_address,
        &mint,
        &destination_address,
        &owner.pubkey(),
        &[],
        amount,
        decimals,
    )
    .expect("Failed to create transfer_checked instruction");

    let message = VersionedMessage::Legacy(Message::new(&[instruction], Some(&owner.pubkey())));
    let tx = VersionedTransaction::try_new(message, &[&owner]).unwrap();

    let resolved_tx = VersionedTransactionResolved::from_kora_built_transaction(&tx)
        .expect("Failed to create resolved transaction");

    let parsed_instructions = IxUtils::parse_token_instructions(&resolved_tx)
        .expect("Failed to parse token instructions");

    let transfers = parsed_instructions
        .get(&ParsedSPLInstructionType::SplTokenTransfer)
        .expect("Expected SplTokenTransfer instructions");

    assert_eq!(transfers.len(), 1);

    match &transfers[0] {
        ParsedSPLInstructionData::SplTokenTransfer {
            amount: parsed_amount,
            owner: parsed_owner,
            multisig_signers,
            mint: parsed_mint,
            source_address: parsed_source,
            destination_address: parsed_destination,
            is_2022,
        } => {
            assert_eq!(*parsed_amount, amount);
            assert_eq!(*parsed_owner, owner.pubkey());
            assert!(multisig_signers.is_empty());
            assert_eq!(*parsed_source, source_address);
            assert_eq!(*parsed_destination, destination_address);
            assert!(*is_2022);
            assert_eq!(parsed_mint.unwrap(), mint);
        }
        _ => panic!("Expected SplTokenTransfer variant"),
    }
}

#[test]
fn test_parse_token_2022_instructions_transfer_checked_with_fee() {
    use crate::transaction::versioned_transaction::VersionedTransactionResolved;
    use solana_message::{Message, VersionedMessage};
    use solana_sdk::{
        signature::{Keypair, Signer},
        transaction::VersionedTransaction,
    };
    use spl_token_2022_interface::extension::transfer_fee::instruction::transfer_checked_with_fee;

    let owner = Keypair::new();
    let source_address = Pubkey::new_unique();
    let mint = Pubkey::new_unique();
    let destination_address = Pubkey::new_unique();
    let amount = 7500u64;
    let decimals = 6u8;

    let instruction = transfer_checked_with_fee(
        &spl_token_2022_interface::ID,
        &source_address,
        &mint,
        &destination_address,
        &owner.pubkey(),
        &[],
        amount,
        decimals,
        0,
    )
    .expect("Failed to create transfer_checked_with_fee instruction");

    let message = VersionedMessage::Legacy(Message::new(&[instruction], Some(&owner.pubkey())));
    let tx = VersionedTransaction::try_new(message, &[&owner]).unwrap();

    let resolved_tx = VersionedTransactionResolved::from_kora_built_transaction(&tx)
        .expect("Failed to create resolved transaction");

    let parsed_instructions = IxUtils::parse_token_instructions(&resolved_tx)
        .expect("Failed to parse token instructions");

    let transfers = parsed_instructions
        .get(&ParsedSPLInstructionType::SplTokenTransfer)
        .expect("Expected SplTokenTransfer instructions");

    assert_eq!(transfers.len(), 1);

    match &transfers[0] {
        ParsedSPLInstructionData::SplTokenTransfer {
            amount: parsed_amount,
            owner: parsed_owner,
            multisig_signers,
            mint: parsed_mint,
            source_address: parsed_source,
            destination_address: parsed_destination,
            is_2022,
        } => {
            assert_eq!(*parsed_amount, amount);
            assert_eq!(*parsed_owner, owner.pubkey());
            assert!(multisig_signers.is_empty());
            assert_eq!(*parsed_source, source_address);
            assert_eq!(*parsed_destination, destination_address);
            assert!(*is_2022);
            assert_eq!(parsed_mint.unwrap(), mint);
        }
        _ => panic!("Expected SplTokenTransfer variant"),
    }
}

#[test]
fn test_parse_token_instructions_burn_checked_requires_three_accounts() {
    use crate::transaction::versioned_transaction::VersionedTransactionResolved;
    use solana_message::{Message, VersionedMessage};
    use solana_sdk::{
        instruction::{AccountMeta, Instruction},
        signature::{Keypair, Signer},
        transaction::VersionedTransaction,
    };

    let payer = Keypair::new();
    let source = Pubkey::new_unique();
    let authority = Pubkey::new_unique();
    let burn_checked_data =
        spl_token_interface::instruction::TokenInstruction::BurnChecked { amount: 10, decimals: 6 }
            .pack();

    let malformed_burn_checked = Instruction {
        program_id: spl_token_interface::ID,
        accounts: vec![
            AccountMeta::new(source, false),
            AccountMeta::new_readonly(authority, false),
        ],
        data: burn_checked_data,
    };

    let message =
        VersionedMessage::Legacy(Message::new(&[malformed_burn_checked], Some(&payer.pubkey())));
    let tx = VersionedTransaction::try_new(message, &[&payer]).unwrap();
    let resolved_tx = VersionedTransactionResolved::from_kora_built_transaction(&tx)
        .expect("Failed to create resolved transaction");

    let result = IxUtils::parse_token_instructions(&resolved_tx);
    assert!(result.is_err(), "BurnChecked with 2 accounts must be rejected");
}

#[test]
fn test_parse_token_2022_instructions_burn_checked_requires_three_accounts() {
    use crate::transaction::versioned_transaction::VersionedTransactionResolved;
    use solana_message::{Message, VersionedMessage};
    use solana_sdk::{
        instruction::{AccountMeta, Instruction},
        signature::{Keypair, Signer},
        transaction::VersionedTransaction,
    };

    let payer = Keypair::new();
    let source = Pubkey::new_unique();
    let authority = Pubkey::new_unique();
    let burn_checked_data = spl_token_2022_interface::instruction::TokenInstruction::BurnChecked {
        amount: 10,
        decimals: 6,
    }
    .pack();

    let malformed_burn_checked = Instruction {
        program_id: spl_token_2022_interface::ID,
        accounts: vec![
            AccountMeta::new(source, false),
            AccountMeta::new_readonly(authority, false),
        ],
        data: burn_checked_data,
    };

    let message =
        VersionedMessage::Legacy(Message::new(&[malformed_burn_checked], Some(&payer.pubkey())));
    let tx = VersionedTransaction::try_new(message, &[&payer]).unwrap();
    let resolved_tx = VersionedTransactionResolved::from_kora_built_transaction(&tx)
        .expect("Failed to create resolved transaction");

    let result = IxUtils::parse_token_instructions(&resolved_tx);
    assert!(result.is_err(), "Token-2022 BurnChecked with 2 accounts must be rejected");
}

#[test]
fn test_parse_token_2022_pausable_pause_records_authority() {
    use crate::transaction::TransactionUtil;
    use solana_message::{Message, VersionedMessage};
    use solana_sdk::signature::{Keypair, Signer};

    let payer = Keypair::new();
    let authority = Pubkey::new_unique();
    let mint = Pubkey::new_unique();

    let ix = spl_token_2022_interface::extension::pausable::instruction::pause(
        &spl_token_2022_interface::id(),
        &mint,
        &authority,
        &[],
    )
    .unwrap();

    let message = VersionedMessage::Legacy(Message::new(&[ix], Some(&payer.pubkey())));
    let resolved_tx = TransactionUtil::new_unsigned_versioned_transaction_resolved(message)
        .expect("Failed to create resolved transaction");

    let result = IxUtils::parse_token_instructions(&resolved_tx);
    assert!(result.is_ok(), "Pausable pause should parse successfully");
    let parsed = result.unwrap();

    let entries = parsed.get(&ParsedSPLInstructionType::SplTokenPause).unwrap();
    assert_eq!(entries.len(), 1);
    if let ParsedSPLInstructionData::SplTokenPause {
        authority: parsed_authority,
        multisig_signers,
    } = &entries[0]
    {
        assert_eq!(*parsed_authority, authority);
        assert!(multisig_signers.is_empty());
    } else {
        panic!("Expected SplTokenPause variant");
    }
}

#[test]
fn test_parse_token_2022_transfer_hook_update_records_authority_and_program_id() {
    use crate::transaction::TransactionUtil;
    use solana_message::{Message, VersionedMessage};
    use solana_sdk::signature::{Keypair, Signer};

    let payer = Keypair::new();
    let mint = Pubkey::new_unique();
    let authority = Pubkey::new_unique();
    let new_program_id = Pubkey::new_unique();

    let ix = spl_token_2022_interface::extension::transfer_hook::instruction::update(
        &spl_token_2022_interface::id(),
        &mint,
        &authority,
        &[],
        Some(new_program_id),
    )
    .unwrap();

    let message = VersionedMessage::Legacy(Message::new(&[ix], Some(&payer.pubkey())));
    let resolved_tx = TransactionUtil::new_unsigned_versioned_transaction_resolved(message)
        .expect("Failed to create resolved transaction");

    let result = IxUtils::parse_token_instructions(&resolved_tx);
    assert!(result.is_ok(), "TransferHook update should parse successfully");
    let parsed = result.unwrap();

    let entries = parsed.get(&ParsedSPLInstructionType::SplTokenTransferHookUpdate).unwrap();
    assert_eq!(entries.len(), 1);
    if let ParsedSPLInstructionData::SplTokenTransferHookUpdate {
        authority: parsed_authority,
        multisig_signers,
        program_id,
    } = &entries[0]
    {
        assert_eq!(*parsed_authority, authority);
        assert!(multisig_signers.is_empty());
        assert_eq!(*program_id, Some(new_program_id));
    } else {
        panic!("Expected SplTokenTransferHookUpdate variant");
    }
}

#[test]
fn test_parse_token_2022_unknown_extension_accounts_captured() {
    use crate::transaction::versioned_transaction::VersionedTransactionResolved;
    use solana_message::{Message, VersionedMessage};
    use solana_sdk::{
        instruction::{AccountMeta, Instruction},
        signature::{Keypair, Signer},
        transaction::VersionedTransaction,
    };

    let payer = Keypair::new();

    let ix = Instruction {
        program_id: spl_token_2022_interface::ID,
        accounts: vec![AccountMeta::new(payer.pubkey(), true)],
        data: spl_token_2022_interface::instruction::TokenInstruction::MemoTransferExtension.pack(),
    };

    let message = VersionedMessage::Legacy(Message::new(&[ix], Some(&payer.pubkey())));
    let tx = VersionedTransaction::try_new(message, &[&payer]).unwrap();
    let resolved_tx = VersionedTransactionResolved::from_kora_built_transaction(&tx)
        .expect("Failed to create resolved transaction");

    let result = IxUtils::parse_token_instructions(&resolved_tx);
    assert!(result.is_ok(), "Unsupported Token-2022 extension should be captured");
    let parsed = result.unwrap();

    let unknown = parsed.get(&ParsedSPLInstructionType::SplTokenUnknownExtension);
    assert!(unknown.is_some(), "Accounts should be captured as SplTokenUnknownExtension");
    let entries = unknown.unwrap();
    assert_eq!(entries.len(), 1);
    if let ParsedSPLInstructionData::SplTokenUnknownExtension { accounts } = &entries[0] {
        assert!(
            accounts.contains(&payer.pubkey()),
            "Fee payer pubkey must be captured so validator can check it"
        );
    } else {
        panic!("Expected SplTokenUnknownExtension variant");
    }
}

#[test]
fn test_parse_spl_token_batch_extracts_inner_transfer() {
    use crate::transaction::versioned_transaction::VersionedTransactionResolved;
    use solana_message::{Message, VersionedMessage};
    use solana_sdk::{
        signature::{Keypair, Signer},
        transaction::VersionedTransaction,
    };

    let payer = Keypair::new();
    let source = Pubkey::new_unique();
    let destination = Pubkey::new_unique();

    let transfer_ix = spl_token_interface::instruction::transfer(
        &spl_token_interface::id(),
        &source,
        &destination,
        &payer.pubkey(),
        &[],
        4242,
    )
    .unwrap();
    let batch_ix =
        spl_token_interface::instruction::batch(&spl_token_interface::id(), &[transfer_ix])
            .unwrap();

    let message = VersionedMessage::Legacy(Message::new(&[batch_ix], Some(&payer.pubkey())));
    let tx = VersionedTransaction::try_new(message, &[&payer]).unwrap();
    let resolved_tx = VersionedTransactionResolved::from_kora_built_transaction(&tx).unwrap();

    let parsed = IxUtils::parse_token_instructions(&resolved_tx).unwrap();
    let transfers = parsed
        .get(&ParsedSPLInstructionType::SplTokenTransfer)
        .expect("Batched transfer must be decoded, not skipped");
    assert_eq!(transfers.len(), 1);
    if let ParsedSPLInstructionData::SplTokenTransfer { amount, owner, is_2022, .. } = &transfers[0]
    {
        assert_eq!(*amount, 4242);
        assert_eq!(*owner, payer.pubkey());
        assert!(!*is_2022);
    } else {
        panic!("Expected SplTokenTransfer variant");
    }
}

#[test]
fn test_parse_spl_token_batch_rejects_nested_batch() {
    use crate::transaction::versioned_transaction::VersionedTransactionResolved;
    use solana_message::{Message, VersionedMessage};
    use solana_sdk::{
        signature::{Keypair, Signer},
        transaction::VersionedTransaction,
    };

    let payer = Keypair::new();
    let source = Pubkey::new_unique();
    let destination = Pubkey::new_unique();

    let transfer_ix = spl_token_interface::instruction::transfer(
        &spl_token_interface::id(),
        &source,
        &destination,
        &payer.pubkey(),
        &[],
        1,
    )
    .unwrap();
    let inner_batch =
        spl_token_interface::instruction::batch(&spl_token_interface::id(), &[transfer_ix])
            .unwrap();
    let outer_batch =
        spl_token_interface::instruction::batch(&spl_token_interface::id(), &[inner_batch])
            .unwrap();

    let message = VersionedMessage::Legacy(Message::new(&[outer_batch], Some(&payer.pubkey())));
    let tx = VersionedTransaction::try_new(message, &[&payer]).unwrap();
    let resolved_tx = VersionedTransactionResolved::from_kora_built_transaction(&tx).unwrap();

    assert!(IxUtils::parse_token_instructions(&resolved_tx).is_err());
}

#[test]
fn test_parse_spl_token_batch_rejects_trailing_accounts() {
    use crate::transaction::versioned_transaction::VersionedTransactionResolved;
    use solana_message::{Message, VersionedMessage};
    use solana_sdk::{
        instruction::AccountMeta,
        signature::{Keypair, Signer},
        transaction::VersionedTransaction,
    };

    let payer = Keypair::new();
    let source = Pubkey::new_unique();
    let destination = Pubkey::new_unique();

    let transfer_ix = spl_token_interface::instruction::transfer(
        &spl_token_interface::id(),
        &source,
        &destination,
        &payer.pubkey(),
        &[],
        1,
    )
    .unwrap();
    let mut batch_ix =
        spl_token_interface::instruction::batch(&spl_token_interface::id(), &[transfer_ix])
            .unwrap();
    batch_ix.accounts.push(AccountMeta::new_readonly(Pubkey::new_unique(), false));

    let message = VersionedMessage::Legacy(Message::new(&[batch_ix], Some(&payer.pubkey())));
    let tx = VersionedTransaction::try_new(message, &[&payer]).unwrap();
    let resolved_tx = VersionedTransactionResolved::from_kora_built_transaction(&tx).unwrap();

    assert!(IxUtils::parse_token_instructions(&resolved_tx).is_err());
}

#[test]
fn test_parse_spl_token_batch_rejects_unrecognized_sub_instruction() {
    use crate::transaction::versioned_transaction::VersionedTransactionResolved;
    use solana_message::{Message, VersionedMessage};
    use solana_sdk::{
        instruction::Instruction,
        signature::{Keypair, Signer},
        transaction::VersionedTransaction,
    };

    let payer = Keypair::new();
    let junk_ix =
        Instruction { program_id: spl_token_interface::id(), accounts: vec![], data: vec![99] };
    let batch_ix =
        spl_token_interface::instruction::batch(&spl_token_interface::id(), &[junk_ix]).unwrap();

    let message = VersionedMessage::Legacy(Message::new(&[batch_ix], Some(&payer.pubkey())));
    let tx = VersionedTransaction::try_new(message, &[&payer]).unwrap();
    let resolved_tx = VersionedTransactionResolved::from_kora_built_transaction(&tx).unwrap();

    assert!(IxUtils::parse_token_instructions(&resolved_tx).is_err());
}

#[test]
fn test_parse_rejects_token_2022_batch() {
    use crate::transaction::versioned_transaction::VersionedTransactionResolved;
    use solana_message::{Message, VersionedMessage};
    use solana_sdk::{
        instruction::Instruction,
        signature::{Keypair, Signer},
        transaction::VersionedTransaction,
    };

    let payer = Keypair::new();
    let fake_t22_batch = Instruction {
        program_id: spl_token_2022_interface::id(),
        accounts: vec![],
        data: vec![BATCH_DISCRIMINATOR],
    };

    let message = VersionedMessage::Legacy(Message::new(&[fake_t22_batch], Some(&payer.pubkey())));
    let tx = VersionedTransaction::try_new(message, &[&payer]).unwrap();
    let resolved_tx = VersionedTransactionResolved::from_kora_built_transaction(&tx).unwrap();

    assert!(IxUtils::parse_token_instructions(&resolved_tx).is_err());
}

#[test]
fn test_parse_spl_token_withdraw_excess_lamports() {
    use crate::transaction::versioned_transaction::VersionedTransactionResolved;
    use solana_message::{Message, VersionedMessage};
    use solana_sdk::{
        signature::{Keypair, Signer},
        transaction::VersionedTransaction,
    };

    let payer = Keypair::new();
    let account = Pubkey::new_unique();
    let destination = Pubkey::new_unique();

    let ix = spl_token_interface::instruction::withdraw_excess_lamports(
        &spl_token_interface::id(),
        &account,
        &destination,
        &payer.pubkey(),
        &[],
    )
    .unwrap();

    let message = VersionedMessage::Legacy(Message::new(&[ix], Some(&payer.pubkey())));
    let tx = VersionedTransaction::try_new(message, &[&payer]).unwrap();
    let resolved_tx = VersionedTransactionResolved::from_kora_built_transaction(&tx).unwrap();

    let parsed = IxUtils::parse_token_instructions(&resolved_tx).unwrap();
    let entries = parsed
        .get(&ParsedSPLInstructionType::SplTokenWithdrawExcessLamports)
        .expect("WithdrawExcessLamports must be parsed");
    assert_eq!(entries.len(), 1);
    if let ParsedSPLInstructionData::SplTokenWithdrawExcessLamports { owner, is_2022, .. } =
        &entries[0]
    {
        assert_eq!(*owner, payer.pubkey());
        assert!(!*is_2022);
    } else {
        panic!("Expected SplTokenWithdrawExcessLamports variant");
    }
}

#[test]
fn test_parse_spl_token_unwrap_lamports() {
    use crate::transaction::versioned_transaction::VersionedTransactionResolved;
    use solana_message::{Message, VersionedMessage};
    use solana_sdk::{
        signature::{Keypair, Signer},
        transaction::VersionedTransaction,
    };

    let payer = Keypair::new();
    let account = Pubkey::new_unique();
    let destination = Pubkey::new_unique();

    let ix = spl_token_interface::instruction::unwrap_lamports(
        &spl_token_interface::id(),
        &account,
        &destination,
        &payer.pubkey(),
        &[],
        Some(500),
    )
    .unwrap();

    let message = VersionedMessage::Legacy(Message::new(&[ix], Some(&payer.pubkey())));
    let tx = VersionedTransaction::try_new(message, &[&payer]).unwrap();
    let resolved_tx = VersionedTransactionResolved::from_kora_built_transaction(&tx).unwrap();

    let parsed = IxUtils::parse_token_instructions(&resolved_tx).unwrap();
    let entries = parsed
        .get(&ParsedSPLInstructionType::SplTokenUnwrapLamports)
        .expect("UnwrapLamports must be parsed");
    assert_eq!(entries.len(), 1);
    if let ParsedSPLInstructionData::SplTokenUnwrapLamports { owner, is_2022, .. } = &entries[0] {
        assert_eq!(*owner, payer.pubkey());
        assert!(!*is_2022);
    } else {
        panic!("Expected SplTokenUnwrapLamports variant");
    }
}

#[test]
fn test_parse_token_2022_unwrap_lamports() {
    use crate::transaction::versioned_transaction::VersionedTransactionResolved;
    use solana_message::{Message, VersionedMessage};
    use solana_sdk::{
        signature::{Keypair, Signer},
        transaction::VersionedTransaction,
    };

    let payer = Keypair::new();
    let account = Pubkey::new_unique();
    let destination = Pubkey::new_unique();

    let ix = spl_token_2022_interface::instruction::unwrap_lamports(
        &spl_token_2022_interface::id(),
        &account,
        &destination,
        &payer.pubkey(),
        &[],
        Some(500),
    )
    .unwrap();

    let message = VersionedMessage::Legacy(Message::new(&[ix], Some(&payer.pubkey())));
    let tx = VersionedTransaction::try_new(message, &[&payer]).unwrap();
    let resolved_tx = VersionedTransactionResolved::from_kora_built_transaction(&tx).unwrap();

    let parsed = IxUtils::parse_token_instructions(&resolved_tx).unwrap();
    let entries = parsed
        .get(&ParsedSPLInstructionType::SplTokenUnwrapLamports)
        .expect("Token-2022 UnwrapLamports must be parsed");
    assert_eq!(entries.len(), 1);
    if let ParsedSPLInstructionData::SplTokenUnwrapLamports { owner, is_2022, .. } = &entries[0] {
        assert_eq!(*owner, payer.pubkey());
        assert!(*is_2022);
    } else {
        panic!("Expected SplTokenUnwrapLamports variant");
    }
}

#[test]
fn test_parse_token_2022_malformed_instruction_rejected() {
    use crate::transaction::versioned_transaction::VersionedTransactionResolved;
    use solana_message::{Message, VersionedMessage};
    use solana_sdk::{
        instruction::{AccountMeta, Instruction},
        signature::{Keypair, Signer},
        transaction::VersionedTransaction,
    };

    let payer = Keypair::new();

    let ix = Instruction {
        program_id: spl_token_2022_interface::ID,
        accounts: vec![AccountMeta::new(payer.pubkey(), true)],
        data: vec![0xFF, 0xFF, 0xFF],
    };

    let message = VersionedMessage::Legacy(Message::new(&[ix], Some(&payer.pubkey())));
    let tx = VersionedTransaction::try_new(message, &[&payer]).unwrap();
    let resolved_tx = VersionedTransactionResolved::from_kora_built_transaction(&tx)
        .expect("Failed to create resolved transaction");

    let result = IxUtils::parse_token_instructions(&resolved_tx);
    assert!(result.is_err(), "Malformed Token-2022 instruction must be rejected");
}

#[test]
fn test_uncompile_instructions() {
    let program_id = Pubkey::new_unique();
    let account1 = Pubkey::new_unique();
    let account2 = Pubkey::new_unique();

    let account_keys = vec![program_id, account1, account2];
    let compiled_ix = CompiledInstruction {
        program_id_index: 0,
        accounts: vec![1, 2], // indices into account_keys
        data: vec![1, 2, 3],
    };

    let instructions = IxUtils::uncompile_instructions(&[compiled_ix], &account_keys).unwrap();

    assert_eq!(instructions.len(), 1);
    let uncompiled = &instructions[0];
    assert_eq!(uncompiled.program_id, program_id);
    assert_eq!(uncompiled.accounts.len(), 2);
    assert_eq!(uncompiled.accounts[0].pubkey, account1);
    assert_eq!(uncompiled.accounts[1].pubkey, account2);
    assert_eq!(uncompiled.data, vec![1, 2, 3]);
}

#[test]
fn test_reconstruct_instruction_from_ui_compiled() {
    let program_id = Pubkey::new_unique();
    let account1 = Pubkey::new_unique();
    let mut account_keys = vec![program_id, account1];

    let ui_compiled = solana_transaction_status_client_types::UiCompiledInstruction {
        program_id_index: 0,
        accounts: vec![1],
        data: bs58::encode(&[1, 2, 3]).into_string(),
        stack_height: None,
    };

    let result = IxUtils::reconstruct_instruction_from_ui(
        &UiInstruction::Compiled(ui_compiled),
        &mut account_keys,
    );

    assert!(result.is_ok());
    let compiled = result.unwrap();
    assert_eq!(compiled.program_id_index, 0);
    assert_eq!(compiled.accounts, vec![1]);
    assert_eq!(compiled.data, vec![1, 2, 3]);
}

#[test]
fn test_reconstruct_partially_decoded_instruction() {
    let program_id = Pubkey::new_unique();
    let account1 = Pubkey::new_unique();
    let account2 = Pubkey::new_unique();
    let mut account_keys = vec![program_id, account1, account2];

    let partial = solana_transaction_status_client_types::UiPartiallyDecodedInstruction {
        program_id: program_id.to_string(),
        accounts: vec![account1.to_string(), account2.to_string()],
        data: bs58::encode(&[5, 6, 7]).into_string(),
        stack_height: None,
    };

    let ui_parsed = UiParsedInstruction::PartiallyDecoded(partial);

    let result = IxUtils::reconstruct_instruction_from_ui(
        &UiInstruction::Parsed(ui_parsed),
        &mut account_keys,
    );

    assert!(result.is_ok());
    let compiled = result.unwrap();
    assert_eq!(compiled.program_id_index, 0);
    assert_eq!(compiled.accounts, vec![1, 2]); // account1, account2 indices
    assert_eq!(compiled.data, vec![5, 6, 7]);
}

#[test]
fn test_reconstruct_partially_decoded_with_cpi_pda_accounts() {
    // Simulate a CPI inner instruction where the PDA authority is NOT in
    // the outer transaction's account keys (e.g. Kamino lending authority).
    let program_id = Pubkey::new_unique();
    let known_account = Pubkey::new_unique();
    let cpi_pda_account = Pubkey::new_unique(); // Not in original keys
    let mut account_keys = vec![program_id, known_account];

    let partial = solana_transaction_status_client_types::UiPartiallyDecodedInstruction {
        program_id: program_id.to_string(),
        accounts: vec![known_account.to_string(), cpi_pda_account.to_string()],
        data: bs58::encode(&[8, 9, 10]).into_string(),
        stack_height: None,
    };

    let ui_parsed = UiParsedInstruction::PartiallyDecoded(partial);

    let result = IxUtils::reconstruct_instruction_from_ui(
        &UiInstruction::Parsed(ui_parsed),
        &mut account_keys,
    );

    assert!(result.is_ok(), "Should succeed even with unknown CPI PDA account");
    let compiled = result.unwrap();
    assert_eq!(compiled.program_id_index, 0);
    // Both accounts should be present — the PDA gets a synthetic index
    assert_eq!(compiled.accounts.len(), 2);
    assert_eq!(compiled.accounts[0], 1); // known_account at index 1
    assert_eq!(compiled.accounts[1], 2); // cpi_pda_account extended at index 2
    assert_eq!(compiled.data, vec![8, 9, 10]);

    // The account_keys vec should have been extended with the PDA
    assert_eq!(account_keys.len(), 3);
    assert_eq!(account_keys[2], cpi_pda_account);
}

#[test]
fn test_reconstruct_partially_decoded_with_unknown_program_id() {
    // When even the program ID is not in the original account keys (CPI to
    // a program not referenced in the outer transaction).
    let known_account = Pubkey::new_unique();
    let cpi_program = Pubkey::new_unique(); // Not in original keys
    let cpi_account = Pubkey::new_unique(); // Not in original keys
    let mut account_keys = vec![known_account];

    let partial = solana_transaction_status_client_types::UiPartiallyDecodedInstruction {
        program_id: cpi_program.to_string(),
        accounts: vec![known_account.to_string(), cpi_account.to_string()],
        data: bs58::encode(&[1]).into_string(),
        stack_height: None,
    };

    let ui_parsed = UiParsedInstruction::PartiallyDecoded(partial);

    let result = IxUtils::reconstruct_instruction_from_ui(
        &UiInstruction::Parsed(ui_parsed),
        &mut account_keys,
    );

    assert!(result.is_ok(), "Should succeed even with unknown program ID");
    let compiled = result.unwrap();
    // Program ID gets index 1, cpi_account gets index 2
    assert_eq!(compiled.program_id_index, 1);
    assert_eq!(compiled.accounts[0], 0); // known_account at original index 0
    assert_eq!(compiled.accounts[1], 2); // cpi_account extended at index 2
    assert_eq!(account_keys.len(), 3);
}

#[test]
fn test_reconstruct_partially_decoded_rejects_program_index_overflow() {
    let mut account_keys: Vec<Pubkey> = (0..256).map(|_| Pubkey::new_unique()).collect();
    let cpi_program = Pubkey::new_unique(); // not in account_keys

    let partial = solana_transaction_status_client_types::UiPartiallyDecodedInstruction {
        program_id: cpi_program.to_string(),
        accounts: vec![account_keys[0].to_string()],
        data: bs58::encode(&[1]).into_string(),
        stack_height: None,
    };

    let ui_parsed = UiParsedInstruction::PartiallyDecoded(partial);

    let result = IxUtils::reconstruct_instruction_from_ui(
        &UiInstruction::Parsed(ui_parsed),
        &mut account_keys,
    );

    assert!(result.is_err(), "should fail when program index would overflow u8");
    assert_eq!(account_keys.len(), 256, "account keys should remain unchanged on failure");
}

#[test]
fn test_reconstruct_partially_decoded_rejects_account_index_overflow() {
    let mut account_keys: Vec<Pubkey> = (0..256).map(|_| Pubkey::new_unique()).collect();
    let program_id = account_keys[0];
    let known_account = account_keys[1];
    let cpi_account = Pubkey::new_unique(); // not in account_keys

    let partial = solana_transaction_status_client_types::UiPartiallyDecodedInstruction {
        program_id: program_id.to_string(),
        accounts: vec![known_account.to_string(), cpi_account.to_string()],
        data: bs58::encode(&[2]).into_string(),
        stack_height: None,
    };

    let ui_parsed = UiParsedInstruction::PartiallyDecoded(partial);

    let result = IxUtils::reconstruct_instruction_from_ui(
        &UiInstruction::Parsed(ui_parsed),
        &mut account_keys,
    );

    assert!(result.is_err(), "should fail when account index would overflow u8");
    assert_eq!(account_keys.len(), 256, "account keys should remain unchanged on failure");
}

#[test]
fn test_reconstruct_partially_decoded_rolls_back_on_mid_loop_overflow() {
    let mut account_keys: Vec<Pubkey> = (0..254).map(|_| Pubkey::new_unique()).collect();
    let program_id = account_keys[0];
    let known_account = account_keys[1];
    let cpi_account_1 = Pubkey::new_unique();
    let cpi_account_2 = Pubkey::new_unique();
    let cpi_account_3 = Pubkey::new_unique(); // triggers overflow after two pushes

    let partial = solana_transaction_status_client_types::UiPartiallyDecodedInstruction {
        program_id: program_id.to_string(),
        accounts: vec![
            known_account.to_string(),
            cpi_account_1.to_string(),
            cpi_account_2.to_string(),
            cpi_account_3.to_string(),
        ],
        data: bs58::encode(&[3]).into_string(),
        stack_height: None,
    };

    let ui_parsed = UiParsedInstruction::PartiallyDecoded(partial);

    let result = IxUtils::reconstruct_instruction_from_ui(
        &UiInstruction::Parsed(ui_parsed),
        &mut account_keys,
    );

    assert!(result.is_err(), "should fail when account index overflows mid-loop");
    assert_eq!(account_keys.len(), 254, "account keys should roll back to snapshot length");
}

#[test]
fn test_reconstruct_system_transfer_instruction() {
    let source = Pubkey::new_unique();
    let destination = Pubkey::new_unique();
    let system_program_id = SYSTEM_PROGRAM_ID;
    let account_keys = vec![system_program_id, source, destination];
    let lamports = 1000000u64;

    let transfer_instruction =
        solana_system_interface::instruction::transfer(&source, &destination, lamports);

    let solana_parsed_transfer = create_parsed_system_transfer(&source, &destination, lamports)
        .expect("Failed to create authentic parsed instruction");

    let result = IxUtils::reconstruct_system_instruction(
        &solana_parsed_transfer,
        &IxUtils::build_account_keys_hashmap(&account_keys),
    );

    assert!(result.is_ok());
    let compiled = result.unwrap();
    assert_eq!(compiled.program_id_index, 0);
    assert_eq!(compiled.accounts, vec![1, 2]); // source, destination indices
    assert_eq!(compiled.data, transfer_instruction.data);
}

#[test]
fn test_reconstruct_system_transfer_with_seed_instruction() {
    let source = Pubkey::new_unique();
    let destination = Pubkey::new_unique();
    let source_base = Pubkey::new_unique();
    let source_owner = Pubkey::new_unique();
    let system_program_id = SYSTEM_PROGRAM_ID;
    let account_keys = vec![system_program_id, source, source_base, destination];
    let lamports = 5000000u64;

    let instruction = solana_system_interface::instruction::transfer_with_seed(
        &source,
        &source_base,
        "test_seed".to_string(),
        &source_owner,
        &destination,
        lamports,
    );

    let solana_parsed = create_parsed_system_transfer_with_seed(
        &source,
        &destination,
        lamports,
        &source_base,
        "test_seed",
        &source_owner,
    )
    .expect("Failed to create parsed instruction");

    let result = IxUtils::reconstruct_system_instruction(
        &solana_parsed,
        &IxUtils::build_account_keys_hashmap(&account_keys),
    );

    assert!(result.is_ok());
    let compiled = result.unwrap();
    assert_eq!(compiled.program_id_index, 0);
    assert_eq!(compiled.accounts, vec![1, 2, 3]); // source, source_base, destination indices
    assert_eq!(compiled.data, instruction.data);
}

#[test]
fn test_reconstruct_system_create_account_instruction() {
    let source = Pubkey::new_unique();
    let new_account = Pubkey::new_unique();
    let owner = Pubkey::new_unique();
    let system_program_id = SYSTEM_PROGRAM_ID;
    let account_keys = vec![system_program_id, source, new_account];
    let lamports = 2000000u64;
    let space = 165u64;

    let instruction = solana_system_interface::instruction::create_account(
        &source,
        &new_account,
        lamports,
        space,
        &owner,
    );

    let solana_parsed =
        create_parsed_system_create_account(&source, &new_account, lamports, space, &owner)
            .expect("Failed to create parsed instruction");

    let result = IxUtils::reconstruct_system_instruction(
        &solana_parsed,
        &IxUtils::build_account_keys_hashmap(&account_keys),
    );

    assert!(result.is_ok());
    let compiled = result.unwrap();
    assert_eq!(compiled.program_id_index, 0);
    assert_eq!(compiled.accounts, vec![1, 2]); // source, new_account indices
    assert_eq!(compiled.data, instruction.data);
}

#[test]
fn test_reconstruct_system_create_account_with_seed_instruction() {
    let source = Pubkey::new_unique();
    let new_account = Pubkey::new_unique();
    let base = Pubkey::new_unique();
    let owner = Pubkey::new_unique();
    let system_program_id = SYSTEM_PROGRAM_ID;
    let account_keys = vec![system_program_id, source, new_account, base];
    let lamports = 3000000u64;
    let space = 200u64;

    let instruction = solana_system_interface::instruction::create_account_with_seed(
        &source,
        &new_account,
        &base,
        "test_seed_create",
        lamports,
        space,
        &owner,
    );

    let solana_parsed = create_parsed_system_create_account_with_seed(
        &source,
        &new_account,
        &base,
        "test_seed_create",
        lamports,
        space,
        &owner,
    )
    .expect("Failed to create parsed instruction");

    let result = IxUtils::reconstruct_system_instruction(
        &solana_parsed,
        &IxUtils::build_account_keys_hashmap(&account_keys),
    );

    assert!(result.is_ok());
    let compiled = result.unwrap();
    assert_eq!(compiled.program_id_index, 0);
    assert_eq!(compiled.accounts, vec![1, 2, 3]); // source, new_account, base indices
    assert_eq!(compiled.data, instruction.data);
}

#[test]
fn test_reconstruct_system_assign_instruction() {
    let account = Pubkey::new_unique();
    let owner = Pubkey::new_unique();
    let system_program_id = SYSTEM_PROGRAM_ID;
    let account_keys = vec![system_program_id, account];

    let instruction = solana_system_interface::instruction::assign(&account, &owner);

    let solana_parsed =
        create_parsed_system_assign(&account, &owner).expect("Failed to create parsed instruction");

    let result = IxUtils::reconstruct_system_instruction(
        &solana_parsed,
        &IxUtils::build_account_keys_hashmap(&account_keys),
    );

    assert!(result.is_ok());
    let compiled = result.unwrap();
    assert_eq!(compiled.program_id_index, 0);
    assert_eq!(compiled.accounts, vec![1]); // account index
    assert_eq!(compiled.data, instruction.data);
}

#[test]
fn test_reconstruct_system_assign_with_seed_instruction() {
    let account = Pubkey::new_unique();
    let base = Pubkey::new_unique();
    let owner = Pubkey::new_unique();
    let system_program_id = SYSTEM_PROGRAM_ID;
    let account_keys = vec![system_program_id, account, base];

    let instruction = solana_system_interface::instruction::assign_with_seed(
        &account,
        &base,
        "test_assign_seed",
        &owner,
    );

    let solana_parsed =
        create_parsed_system_assign_with_seed(&account, &base, "test_assign_seed", &owner)
            .expect("Failed to create parsed instruction");

    let result = IxUtils::reconstruct_system_instruction(
        &solana_parsed,
        &IxUtils::build_account_keys_hashmap(&account_keys),
    );

    assert!(result.is_ok());
    let compiled = result.unwrap();
    assert_eq!(compiled.program_id_index, 0);
    assert_eq!(compiled.accounts, vec![1, 2]); // account, base indices
    assert_eq!(compiled.data, instruction.data);
}

#[test]
fn test_reconstruct_system_withdraw_nonce_account_instruction() {
    let nonce_account = Pubkey::new_unique();
    let recipient = Pubkey::new_unique();
    let nonce_authority = Pubkey::new_unique();
    let system_program_id = SYSTEM_PROGRAM_ID;
    let account_keys = vec![system_program_id, nonce_account, recipient, nonce_authority];
    let lamports = 1500000u64;

    let instruction = solana_system_interface::instruction::withdraw_nonce_account(
        &nonce_account,
        &nonce_authority,
        &recipient,
        lamports,
    );

    let solana_parsed = create_parsed_system_withdraw_nonce_account(
        &nonce_account,
        &nonce_authority,
        &recipient,
        lamports,
    )
    .expect("Failed to create parsed instruction");

    let result = IxUtils::reconstruct_system_instruction(
        &solana_parsed,
        &IxUtils::build_account_keys_hashmap(&account_keys),
    );

    assert!(result.is_ok());
    let compiled = result.unwrap();
    assert_eq!(compiled.program_id_index, 0);
    assert_eq!(compiled.accounts, vec![1, 2, 3]); // nonce_account, recipient, nonce_authority indices
    assert_eq!(compiled.data, instruction.data);
}

#[test]
fn test_reconstruct_spl_token_transfer_instruction() {
    let source = Pubkey::new_unique();
    let destination = Pubkey::new_unique();
    let authority = Pubkey::new_unique();
    let token_program_id = spl_token_interface::ID;
    let account_keys = vec![token_program_id, source, destination, authority];
    let amount = 1000000u64;

    let transfer_instruction = spl_token_interface::instruction::transfer(
        &spl_token_interface::ID,
        &source,
        &destination,
        &authority,
        &[],
        amount,
    )
    .expect("Failed to create transfer instruction");

    let solana_parsed_transfer =
        create_parsed_spl_token_transfer(&source, &destination, &authority, amount)
            .expect("Failed to create parsed instruction");

    let result = IxUtils::reconstruct_spl_token_instruction(
        &solana_parsed_transfer,
        &IxUtils::build_account_keys_hashmap(&account_keys),
    );

    assert!(result.is_ok());
    let compiled = result.unwrap();
    assert_eq!(compiled.program_id_index, 0);
    assert_eq!(compiled.accounts, vec![1, 2, 3]); // source, destination, authority indices
    assert_eq!(compiled.data, transfer_instruction.data);
}

#[test]
fn test_reconstruct_spl_token_batch_instruction() {
    let source = Pubkey::new_unique();
    let destination = Pubkey::new_unique();
    let authority = Pubkey::new_unique();
    let token_program_id = spl_token_interface::ID;
    let account_keys = vec![token_program_id, source, destination, authority];
    let amount = 1000000u64;

    let transfer_instruction = spl_token_interface::instruction::transfer(
        &spl_token_interface::ID,
        &source,
        &destination,
        &authority,
        &[],
        amount,
    )
    .expect("Failed to create transfer instruction");
    let expected_batch =
        spl_token_interface::instruction::batch(&spl_token_interface::ID, &[transfer_instruction])
            .expect("Failed to create batch instruction");

    let message = Message::new(&[expected_batch.clone()], None);
    let account_keys_for_parsing = AccountKeys::new(&message.account_keys, None);
    let parsed_batch = parse_instruction::parse(
        &spl_token_interface::ID,
        &message.instructions[0],
        &account_keys_for_parsing,
        None,
    )
    .expect("Failed to parse batch instruction");

    let compiled = IxUtils::reconstruct_spl_token_instruction(
        &parsed_batch,
        &IxUtils::build_account_keys_hashmap(&account_keys),
    )
    .expect("Failed to reconstruct batch instruction");

    assert_eq!(compiled.program_id_index, 0);
    assert_eq!(compiled.accounts, vec![1, 2, 3]);
    assert_eq!(compiled.data, expected_batch.data);
}

#[test]
fn test_reconstruct_spl_token_transfer_checked_instruction() {
    let source = Pubkey::new_unique();
    let destination = Pubkey::new_unique();
    let authority = Pubkey::new_unique();
    let mint = Pubkey::new_unique();
    let token_program_id = spl_token_interface::ID;
    let account_keys = vec![token_program_id, source, mint, destination, authority];
    let amount = 2000000u64;
    let decimals = 6u8;

    let instruction = spl_token_interface::instruction::transfer_checked(
        &spl_token_interface::ID,
        &source,
        &mint,
        &destination,
        &authority,
        &[],
        amount,
        decimals,
    )
    .expect("Failed to create transfer_checked instruction");

    let solana_parsed = create_parsed_spl_token_transfer_checked(
        &source,
        &mint,
        &destination,
        &authority,
        amount,
        decimals,
    )
    .expect("Failed to create parsed instruction");

    let result = IxUtils::reconstruct_spl_token_instruction(
        &solana_parsed,
        &IxUtils::build_account_keys_hashmap(&account_keys),
    );

    assert!(result.is_ok());
    let compiled = result.unwrap();
    assert_eq!(compiled.program_id_index, 0);
    assert_eq!(compiled.accounts, vec![1, 2, 3, 4]); // source, mint, destination, authority indices
    assert_eq!(compiled.data, instruction.data);
}

#[test]
fn test_reconstruct_spl_token_burn_instruction() {
    let account = Pubkey::new_unique();
    let mint = Pubkey::new_unique();
    let authority = Pubkey::new_unique();
    let token_program_id = spl_token_interface::ID;
    let account_keys = vec![token_program_id, account, mint, authority];
    let amount = 500000u64;

    let instruction = spl_token_interface::instruction::burn(
        &spl_token_interface::ID,
        &account,
        &mint,
        &authority,
        &[],
        amount,
    )
    .expect("Failed to create burn instruction");

    let solana_parsed = create_parsed_spl_token_burn(&account, &mint, &authority, amount)
        .expect("Failed to create parsed instruction");

    let result = IxUtils::reconstruct_spl_token_instruction(
        &solana_parsed,
        &IxUtils::build_account_keys_hashmap(&account_keys),
    );

    assert!(result.is_ok());
    let compiled = result.unwrap();
    assert_eq!(compiled.program_id_index, 0);
    assert_eq!(compiled.accounts, vec![1, 2, 3]); // account, mint, authority indices (mint included when present in parsed data)
    assert_eq!(compiled.data, instruction.data);
}

#[test]
fn test_reconstruct_spl_token_burn_instruction_falls_back_without_mint_index() {
    let account = Pubkey::new_unique();
    let mint = Pubkey::new_unique();
    let authority = Pubkey::new_unique();
    let token_program_id = spl_token_interface::ID;
    // Deliberately omit mint from account keys to validate fallback behavior.
    let account_keys = vec![token_program_id, account, authority];
    let amount = 500000u64;

    let instruction = spl_token_interface::instruction::burn(
        &spl_token_interface::ID,
        &account,
        &mint,
        &authority,
        &[],
        amount,
    )
    .expect("Failed to create burn instruction");

    let solana_parsed = create_parsed_spl_token_burn(&account, &mint, &authority, amount)
        .expect("Failed to create parsed instruction");

    let result = IxUtils::reconstruct_spl_token_instruction(
        &solana_parsed,
        &IxUtils::build_account_keys_hashmap(&account_keys),
    );

    assert!(result.is_ok());
    let compiled = result.unwrap();
    assert_eq!(compiled.program_id_index, 0);
    assert_eq!(compiled.accounts, vec![1, 2]); // account, authority fallback
    assert_eq!(compiled.data, instruction.data);
}

#[test]
fn test_reconstruct_spl_token_burn_checked_instruction() {
    let account = Pubkey::new_unique();
    let authority = Pubkey::new_unique();
    let mint = Pubkey::new_unique();
    let token_program_id = spl_token_interface::ID;
    let account_keys = vec![token_program_id, account, mint, authority];
    let amount = 750000u64;
    let decimals = 6u8;

    let instruction = spl_token_interface::instruction::burn_checked(
        &spl_token_interface::ID,
        &account,
        &mint,
        &authority,
        &[],
        amount,
        decimals,
    )
    .expect("Failed to create burn_checked instruction");

    let solana_parsed =
        create_parsed_spl_token_burn_checked(&account, &mint, &authority, amount, decimals)
            .expect("Failed to create parsed instruction");

    let result = IxUtils::reconstruct_spl_token_instruction(
        &solana_parsed,
        &IxUtils::build_account_keys_hashmap(&account_keys),
    );

    assert!(result.is_ok());
    let compiled = result.unwrap();
    assert_eq!(compiled.program_id_index, 0);
    assert_eq!(compiled.accounts, vec![1, 2, 3]); // account, mint, authority indices
    assert_eq!(compiled.data, instruction.data);
}

#[test]
fn test_reconstruct_spl_token_close_account_instruction() {
    let account = Pubkey::new_unique();
    let destination = Pubkey::new_unique();
    let authority = Pubkey::new_unique();
    let token_program_id = spl_token_interface::ID;
    let account_keys = vec![token_program_id, account, destination, authority];

    let instruction = spl_token_interface::instruction::close_account(
        &spl_token_interface::ID,
        &account,
        &destination,
        &authority,
        &[],
    )
    .expect("Failed to create close_account instruction");

    let solana_parsed = create_parsed_spl_token_close_account(&account, &destination, &authority)
        .expect("Failed to create parsed instruction");

    let result = IxUtils::reconstruct_spl_token_instruction(
        &solana_parsed,
        &IxUtils::build_account_keys_hashmap(&account_keys),
    );

    assert!(result.is_ok());
    let compiled = result.unwrap();
    assert_eq!(compiled.program_id_index, 0);
    assert_eq!(compiled.accounts, vec![1, 2, 3]); // account, destination, authority indices
    assert_eq!(compiled.data, instruction.data);
}

#[test]
fn test_reconstruct_spl_token_set_authority_on_token_account() {
    let token_account = Pubkey::new_unique();
    let new_authority = Pubkey::new_unique();
    let authority = Pubkey::new_unique();
    let token_program_id = spl_token_interface::ID;
    let account_keys = vec![token_program_id, token_account, authority];

    // AccountOwner is account-level, so the parser emits the target under `account`.
    let solana_parsed = create_parsed_spl_token_set_authority(
        &token_account,
        spl_token_interface::instruction::AuthorityType::AccountOwner,
        &new_authority,
        &authority,
    )
    .expect("Failed to create parsed instruction");

    let result = IxUtils::reconstruct_spl_token_instruction(
        &solana_parsed,
        &IxUtils::build_account_keys_hashmap(&account_keys),
    );

    assert!(result.is_ok(), "account-level setAuthority should reconstruct: {:?}", result.err());
    let compiled = result.unwrap();
    assert_eq!(compiled.program_id_index, 0);
    assert_eq!(compiled.accounts, vec![1, 2]); // target account, current authority indices
}

#[test]
fn test_reconstruct_spl_token_set_authority_on_mint() {
    let mint = Pubkey::new_unique();
    let new_authority = Pubkey::new_unique();
    let authority = Pubkey::new_unique();
    let token_program_id = spl_token_interface::ID;
    let account_keys = vec![token_program_id, mint, authority];

    // FreezeAccount is mint-level, so the parser emits the target under `mint`
    // instead of `account` (agave parse_token.rs, TokenInstruction::SetAuthority).
    let solana_parsed = create_parsed_spl_token_set_authority(
        &mint,
        spl_token_interface::instruction::AuthorityType::FreezeAccount,
        &new_authority,
        &authority,
    )
    .expect("Failed to create parsed instruction");

    let result = IxUtils::reconstruct_spl_token_instruction(
        &solana_parsed,
        &IxUtils::build_account_keys_hashmap(&account_keys),
    );

    assert!(result.is_ok(), "mint-level setAuthority should reconstruct: {:?}", result.err());
    let compiled = result.unwrap();
    assert_eq!(compiled.program_id_index, 0);
    assert_eq!(compiled.accounts, vec![1, 2]); // mint, current authority indices
}

#[test]
fn test_reconstruct_spl_token_sync_native_instruction() {
    let account = Pubkey::new_unique();
    let token_program_id = spl_token_interface::ID;
    let account_keys = vec![token_program_id, account];

    let instruction =
        spl_token_interface::instruction::sync_native(&spl_token_interface::ID, &account)
            .expect("Failed to create sync_native instruction");

    let solana_parsed =
        create_parsed_spl_token_sync_native(&account).expect("Failed to create parsed instruction");

    let result = IxUtils::reconstruct_spl_token_instruction(
        &solana_parsed,
        &IxUtils::build_account_keys_hashmap(&account_keys),
    );

    assert!(result.is_ok(), "syncNative CPI should reconstruct: {:?}", result.err());
    let compiled = result.unwrap();
    assert_eq!(compiled.program_id_index, 0);
    assert_eq!(compiled.accounts, vec![1]);
    assert_eq!(compiled.data, instruction.data);
}

#[test]
fn test_reconstruct_spl_token_approve_instruction() {
    let source = Pubkey::new_unique();
    let delegate = Pubkey::new_unique();
    let owner = Pubkey::new_unique();
    let token_program_id = spl_token_interface::ID;
    let account_keys = vec![token_program_id, source, delegate, owner];
    let amount = 1000000u64;

    let instruction = spl_token_interface::instruction::approve(
        &spl_token_interface::ID,
        &source,
        &delegate,
        &owner,
        &[],
        amount,
    )
    .expect("Failed to create approve instruction");

    let solana_parsed = create_parsed_spl_token_approve(&source, &delegate, &owner, amount)
        .expect("Failed to create parsed instruction");

    let result = IxUtils::reconstruct_spl_token_instruction(
        &solana_parsed,
        &IxUtils::build_account_keys_hashmap(&account_keys),
    );

    assert!(result.is_ok());
    let compiled = result.unwrap();
    assert_eq!(compiled.program_id_index, 0);
    assert_eq!(compiled.accounts, vec![1, 2, 3]); // source, delegate, owner indices
    assert_eq!(compiled.data, instruction.data);
}

#[test]
fn test_reconstruct_spl_token_approve_checked_instruction() {
    let source = Pubkey::new_unique();
    let delegate = Pubkey::new_unique();
    let owner = Pubkey::new_unique();
    let mint = Pubkey::new_unique();
    let token_program_id = spl_token_interface::ID;
    let account_keys = vec![token_program_id, source, mint, delegate, owner];
    let amount = 2500000u64;
    let decimals = 6u8;

    let instruction = spl_token_interface::instruction::approve_checked(
        &spl_token_interface::ID,
        &source,
        &mint,
        &delegate,
        &owner,
        &[],
        amount,
        decimals,
    )
    .expect("Failed to create approve_checked instruction");

    let solana_parsed = create_parsed_spl_token_approve_checked(
        &source, &mint, &delegate, &owner, amount, decimals,
    )
    .expect("Failed to create parsed instruction");

    let result = IxUtils::reconstruct_spl_token_instruction(
        &solana_parsed,
        &IxUtils::build_account_keys_hashmap(&account_keys),
    );

    assert!(result.is_ok());
    let compiled = result.unwrap();
    assert_eq!(compiled.program_id_index, 0);
    assert_eq!(compiled.accounts, vec![1, 2, 3, 4]); // source, mint, delegate, owner indices
    assert_eq!(compiled.data, instruction.data);
}

#[test]
fn test_reconstruct_spl_token_initialize_multisig_preserves_signers_and_threshold() {
    let multisig = Pubkey::new_unique();
    let fee_payer = Pubkey::new_unique();
    let other_signer = Pubkey::new_unique();
    let m = 1u8;

    let real_ix = spl_token_interface::instruction::initialize_multisig(
        &spl_token_interface::ID,
        &multisig,
        &[&fee_payer, &other_signer],
        m,
    )
    .expect("Failed to create initialize_multisig instruction");

    let message = Message::new(std::slice::from_ref(&real_ix), None);
    let account_keys_for_parsing = AccountKeys::new(&message.account_keys, None);
    let parsed = parse_instruction::parse(
        &spl_token_interface::ID,
        &message.instructions[0],
        &account_keys_for_parsing,
        None,
    )
    .expect("Failed to parse initialize_multisig instruction");

    let account_keys = message.account_keys.clone();
    let compiled = IxUtils::reconstruct_spl_token_instruction(
        &parsed,
        &IxUtils::build_account_keys_hashmap(&account_keys),
    )
    .expect("Failed to reconstruct initialize_multisig instruction");

    assert_eq!(compiled.data, real_ix.data);

    let unpacked =
        spl_token_interface::instruction::TokenInstruction::unpack(&compiled.data).unwrap();
    assert!(matches!(
        unpacked,
        spl_token_interface::instruction::TokenInstruction::InitializeMultisig { m: got }
            if got == m
    ));

    let reconstructed_signers: Vec<Pubkey> =
        compiled.accounts[2..].iter().map(|i| account_keys[*i as usize]).collect();
    assert!(reconstructed_signers.contains(&fee_payer));
    assert!(reconstructed_signers.contains(&other_signer));
}

#[test]
fn test_reconstruct_spl_token_initialize_multisig2_preserves_signers_and_threshold() {
    let multisig = Pubkey::new_unique();
    let fee_payer = Pubkey::new_unique();
    let other_signer = Pubkey::new_unique();
    let m = 2u8;

    let real_ix = spl_token_interface::instruction::initialize_multisig2(
        &spl_token_interface::ID,
        &multisig,
        &[&fee_payer, &other_signer],
        m,
    )
    .expect("Failed to create initialize_multisig2 instruction");

    let message = Message::new(std::slice::from_ref(&real_ix), None);
    let account_keys_for_parsing = AccountKeys::new(&message.account_keys, None);
    let parsed = parse_instruction::parse(
        &spl_token_interface::ID,
        &message.instructions[0],
        &account_keys_for_parsing,
        None,
    )
    .expect("Failed to parse initialize_multisig2 instruction");

    let account_keys = message.account_keys.clone();
    let compiled = IxUtils::reconstruct_spl_token_instruction(
        &parsed,
        &IxUtils::build_account_keys_hashmap(&account_keys),
    )
    .expect("Failed to reconstruct initialize_multisig2 instruction");

    assert_eq!(compiled.data, real_ix.data);

    let unpacked =
        spl_token_interface::instruction::TokenInstruction::unpack(&compiled.data).unwrap();
    assert!(matches!(
        unpacked,
        spl_token_interface::instruction::TokenInstruction::InitializeMultisig2 { m: got }
            if got == m
    ));

    // InitializeMultisig2 has no rent sysvar, so signer accounts start at index 1.
    let reconstructed_signers: Vec<Pubkey> =
        compiled.accounts[1..].iter().map(|i| account_keys[*i as usize]).collect();
    assert!(reconstructed_signers.contains(&fee_payer));
    assert!(reconstructed_signers.contains(&other_signer));
}

#[test]
fn test_reconstruct_spl_token_initialize_mint_preserves_data_and_rent() {
    let mint = Pubkey::new_unique();
    let mint_authority = Pubkey::new_unique();
    let freeze_authority = Pubkey::new_unique();
    let decimals = 6u8;

    let real_ix = spl_token_interface::instruction::initialize_mint(
        &spl_token_interface::ID,
        &mint,
        &mint_authority,
        Some(&freeze_authority),
        decimals,
    )
    .expect("Failed to create initialize_mint instruction");

    let message = Message::new(std::slice::from_ref(&real_ix), None);
    let account_keys_for_parsing = AccountKeys::new(&message.account_keys, None);
    let parsed = parse_instruction::parse(
        &spl_token_interface::ID,
        &message.instructions[0],
        &account_keys_for_parsing,
        None,
    )
    .expect("Failed to parse initialize_mint instruction");

    let account_keys = message.account_keys.clone();
    let compiled = IxUtils::reconstruct_spl_token_instruction(
        &parsed,
        &IxUtils::build_account_keys_hashmap(&account_keys),
    )
    .expect("Failed to reconstruct initialize_mint instruction");

    assert_eq!(compiled.data, real_ix.data);
    assert_eq!(compiled.accounts.len(), real_ix.accounts.len());
    assert!(matches!(
        spl_token_interface::instruction::TokenInstruction::unpack(&compiled.data).unwrap(),
        spl_token_interface::instruction::TokenInstruction::InitializeMint { .. }
    ));
}

#[test]
fn test_reconstruct_spl_token_initialize_mint2_preserves_data() {
    let mint = Pubkey::new_unique();
    let mint_authority = Pubkey::new_unique();
    let decimals = 9u8;

    let real_ix = spl_token_interface::instruction::initialize_mint2(
        &spl_token_interface::ID,
        &mint,
        &mint_authority,
        None,
        decimals,
    )
    .expect("Failed to create initialize_mint2 instruction");

    let message = Message::new(std::slice::from_ref(&real_ix), None);
    let account_keys_for_parsing = AccountKeys::new(&message.account_keys, None);
    let parsed = parse_instruction::parse(
        &spl_token_interface::ID,
        &message.instructions[0],
        &account_keys_for_parsing,
        None,
    )
    .expect("Failed to parse initialize_mint2 instruction");

    let account_keys = message.account_keys.clone();
    let compiled = IxUtils::reconstruct_spl_token_instruction(
        &parsed,
        &IxUtils::build_account_keys_hashmap(&account_keys),
    )
    .expect("Failed to reconstruct initialize_mint2 instruction");

    assert_eq!(compiled.data, real_ix.data);
    assert!(matches!(
        spl_token_interface::instruction::TokenInstruction::unpack(&compiled.data).unwrap(),
        spl_token_interface::instruction::TokenInstruction::InitializeMint2 { .. }
    ));
}

#[test]
fn test_reconstruct_spl_token_set_authority_unpacks_and_preserves_new_authority() {
    let account = Pubkey::new_unique();
    let current_authority = Pubkey::new_unique();
    let new_authority = Pubkey::new_unique();

    let real_ix = spl_token_interface::instruction::set_authority(
        &spl_token_interface::ID,
        &account,
        Some(&new_authority),
        spl_token_interface::instruction::AuthorityType::MintTokens,
        &current_authority,
        &[],
    )
    .expect("Failed to create set_authority instruction");

    let message = Message::new(std::slice::from_ref(&real_ix), None);
    let account_keys_for_parsing = AccountKeys::new(&message.account_keys, None);
    let parsed = parse_instruction::parse(
        &spl_token_interface::ID,
        &message.instructions[0],
        &account_keys_for_parsing,
        None,
    )
    .expect("Failed to parse set_authority instruction");

    let account_keys = message.account_keys.clone();
    let compiled = IxUtils::reconstruct_spl_token_instruction(
        &parsed,
        &IxUtils::build_account_keys_hashmap(&account_keys),
    )
    .expect("Failed to reconstruct set_authority instruction");

    // The defect was a discriminator-only reconstruction that failed to unpack, silently
    // skipping the allow_set_authority gate. It must now unpack, and the gate-relevant
    // new_authority must survive; authority_type is not read downstream.
    match spl_token_interface::instruction::TokenInstruction::unpack(&compiled.data)
        .expect("reconstructed SetAuthority must unpack")
    {
        spl_token_interface::instruction::TokenInstruction::SetAuthority {
            new_authority: got,
            ..
        } => assert_eq!(Option::<Pubkey>::from(got), Some(new_authority)),
        other => panic!("expected SetAuthority, got {:?}", other),
    }
}

#[test]
fn test_reconstruct_spl_token_initialize_account2_preserves_owner() {
    let account = Pubkey::new_unique();
    let mint = Pubkey::new_unique();
    let owner = Pubkey::new_unique();

    let real_ix = spl_token_interface::instruction::initialize_account2(
        &spl_token_interface::ID,
        &account,
        &mint,
        &owner,
    )
    .expect("Failed to create initialize_account2 instruction");

    let message = Message::new(std::slice::from_ref(&real_ix), None);
    let account_keys_for_parsing = AccountKeys::new(&message.account_keys, None);
    let parsed = parse_instruction::parse(
        &spl_token_interface::ID,
        &message.instructions[0],
        &account_keys_for_parsing,
        None,
    )
    .expect("Failed to parse initialize_account2 instruction");

    let account_keys = message.account_keys.clone();
    let compiled = IxUtils::reconstruct_spl_token_instruction(
        &parsed,
        &IxUtils::build_account_keys_hashmap(&account_keys),
    )
    .expect("Failed to reconstruct initialize_account2 instruction");

    assert_eq!(compiled.data, real_ix.data);
    assert_eq!(
        compiled.accounts.len(),
        instruction_indexes::spl_token_initialize_account2::REQUIRED_NUMBER_OF_ACCOUNTS
    );

    let unpacked =
        spl_token_interface::instruction::TokenInstruction::unpack(&compiled.data).unwrap();
    assert!(matches!(
        unpacked,
        spl_token_interface::instruction::TokenInstruction::InitializeAccount2 { owner: got }
            if got == owner
    ));
}

#[test]
fn test_reconstruct_token2022_transfer_instruction() {
    let source = Pubkey::new_unique();
    let destination = Pubkey::new_unique();
    let authority = Pubkey::new_unique();
    let token_program_id = spl_token_2022_interface::ID;
    let account_keys = vec![token_program_id, source, destination, authority];
    let amount = 1500000u64;

    #[allow(deprecated)]
    let instruction = spl_token_2022_interface::instruction::transfer(
        &spl_token_2022_interface::ID,
        &source,
        &destination,
        &authority,
        &[],
        amount,
    )
    .expect("Failed to create transfer instruction");

    let solana_parsed = create_parsed_token2022_transfer(&source, &destination, &authority, amount)
        .expect("Failed to create parsed instruction");

    let result = IxUtils::reconstruct_spl_token_instruction(
        &solana_parsed,
        &IxUtils::build_account_keys_hashmap(&account_keys),
    );

    assert!(result.is_ok());
    let compiled = result.unwrap();
    assert_eq!(compiled.program_id_index, 0);
    assert_eq!(compiled.accounts, vec![1, 2, 3]); // source, destination, authority indices
    assert_eq!(compiled.data, instruction.data);
}

#[test]
fn test_reconstruct_token2022_transfer_checked_instruction() {
    let source = Pubkey::new_unique();
    let destination = Pubkey::new_unique();
    let authority = Pubkey::new_unique();
    let mint = Pubkey::new_unique();
    let token_program_id = spl_token_2022_interface::ID;
    let account_keys = vec![token_program_id, source, mint, destination, authority];
    let amount = 3000000u64;
    let decimals = 6u8;

    let instruction = spl_token_2022_interface::instruction::transfer_checked(
        &spl_token_2022_interface::ID,
        &source,
        &mint,
        &destination,
        &authority,
        &[],
        amount,
        decimals,
    )
    .expect("Failed to create transfer_checked instruction");

    let solana_parsed = create_parsed_token2022_transfer_checked(
        &source,
        &mint,
        &destination,
        &authority,
        amount,
        decimals,
    )
    .expect("Failed to create parsed instruction");

    let result = IxUtils::reconstruct_spl_token_instruction(
        &solana_parsed,
        &IxUtils::build_account_keys_hashmap(&account_keys),
    );

    assert!(result.is_ok());
    let compiled = result.unwrap();
    assert_eq!(compiled.program_id_index, 0);
    assert_eq!(compiled.accounts, vec![1, 2, 3, 4]); // source, mint, destination, authority indices
    assert_eq!(compiled.data, instruction.data);
}

#[test]
fn test_reconstruct_token2022_get_account_data_size_instruction() {
    let mint = Pubkey::new_unique();
    let token_program_id = spl_token_2022_interface::ID;
    let account_keys = vec![token_program_id, mint];

    let instruction = spl_token_2022_interface::instruction::get_account_data_size(
        &spl_token_2022_interface::ID,
        &mint,
        &[],
    )
    .expect("Failed to create get_account_data_size instruction");

    let solana_parsed = create_parsed_token2022_get_account_data_size(&mint, &[])
        .expect("Failed to create parsed instruction");

    let result = IxUtils::reconstruct_spl_token_instruction(
        &solana_parsed,
        &IxUtils::build_account_keys_hashmap(&account_keys),
    );

    assert!(result.is_ok());
    let compiled = result.unwrap();
    assert_eq!(compiled.program_id_index, 0);
    assert_eq!(compiled.accounts, vec![1]); // mint index
    assert_eq!(compiled.data, instruction.data);
}

fn create_parsed_token2022_initialize_account3(
    account: &Pubkey,
    mint: &Pubkey,
    owner: &Pubkey,
) -> Result<solana_transaction_status_client_types::ParsedInstruction, Box<dyn std::error::Error>> {
    let solana_instruction = spl_token_2022_interface::instruction::initialize_account3(
        &spl_token_2022_interface::ID,
        account,
        mint,
        owner,
    )?;

    let message = Message::new(&[solana_instruction], None);
    let compiled_instruction = &message.instructions[0];

    let account_keys_for_parsing = AccountKeys::new(&message.account_keys, None);

    let parsed = parse_instruction::parse(
        &spl_token_2022_interface::ID,
        compiled_instruction,
        &account_keys_for_parsing,
        None,
    )?;

    Ok(parsed)
}

#[test]
fn test_reconstruct_token2022_get_account_data_size_instruction_with_extensions() {
    let mint = Pubkey::new_unique();
    let token_program_id = spl_token_2022_interface::ID;
    let account_keys = vec![token_program_id, mint];
    let extension_types = vec![
        spl_token_2022_interface::extension::ExtensionType::ImmutableOwner,
        spl_token_2022_interface::extension::ExtensionType::TransferFeeAmount,
    ];

    let instruction = spl_token_2022_interface::instruction::get_account_data_size(
        &spl_token_2022_interface::ID,
        &mint,
        &extension_types,
    )
    .expect("Failed to create get_account_data_size instruction");

    let solana_parsed = create_parsed_token2022_get_account_data_size(&mint, &extension_types)
        .expect("Failed to create parsed instruction");

    let result = IxUtils::reconstruct_spl_token_instruction(
        &solana_parsed,
        &IxUtils::build_account_keys_hashmap(&account_keys),
    );

    assert!(result.is_ok());
    let compiled = result.unwrap();
    assert_eq!(compiled.program_id_index, 0);
    assert_eq!(compiled.accounts, vec![1]);
    assert_eq!(compiled.data, instruction.data);
}

#[test]
fn test_reconstruct_token2022_initialize_account3_instruction() {
    let account = Pubkey::new_unique();
    let mint = Pubkey::new_unique();
    let owner = Pubkey::new_unique();
    let token_program_id = spl_token_2022_interface::ID;
    let account_keys = vec![token_program_id, account, mint];

    let instruction = spl_token_2022_interface::instruction::initialize_account3(
        &spl_token_2022_interface::ID,
        &account,
        &mint,
        &owner,
    )
    .expect("Failed to create initialize_account3 instruction");

    let solana_parsed = create_parsed_token2022_initialize_account3(&account, &mint, &owner)
        .expect("Failed to create parsed instruction");

    let result = IxUtils::reconstruct_spl_token_instruction(
        &solana_parsed,
        &IxUtils::build_account_keys_hashmap(&account_keys),
    );

    assert!(result.is_ok());
    let compiled = result.unwrap();
    assert_eq!(compiled.program_id_index, 0);
    assert_eq!(compiled.accounts, vec![1, 2]);
    assert_eq!(compiled.data, instruction.data);
}

#[test]
fn test_reconstruct_token2022_burn_instruction() {
    let account = Pubkey::new_unique();
    let mint = Pubkey::new_unique();
    let authority = Pubkey::new_unique();
    let token_program_id = spl_token_2022_interface::ID;
    let account_keys = vec![token_program_id, account, mint, authority];
    let amount = 800000u64;

    let instruction = spl_token_2022_interface::instruction::burn(
        &spl_token_2022_interface::ID,
        &account,
        &mint,
        &authority,
        &[],
        amount,
    )
    .expect("Failed to create burn instruction");

    let solana_parsed = create_parsed_token2022_burn(&account, &mint, &authority, amount)
        .expect("Failed to create parsed instruction");

    let result = IxUtils::reconstruct_spl_token_instruction(
        &solana_parsed,
        &IxUtils::build_account_keys_hashmap(&account_keys),
    );

    assert!(result.is_ok());
    let compiled = result.unwrap();
    assert_eq!(compiled.program_id_index, 0);
    assert_eq!(compiled.accounts, vec![1, 2, 3]); // account, mint, authority indices (mint included when present in parsed data)
    assert_eq!(compiled.data, instruction.data);
}

#[test]
fn test_reconstruct_token2022_burn_checked_instruction() {
    let account = Pubkey::new_unique();
    let authority = Pubkey::new_unique();
    let mint = Pubkey::new_unique();
    let token_program_id = spl_token_2022_interface::ID;
    let account_keys = vec![token_program_id, account, mint, authority];
    let amount = 900000u64;
    let decimals = 6u8;

    let instruction = spl_token_2022_interface::instruction::burn_checked(
        &spl_token_2022_interface::ID,
        &account,
        &mint,
        &authority,
        &[],
        amount,
        decimals,
    )
    .expect("Failed to create burn_checked instruction");

    let solana_parsed =
        create_parsed_token2022_burn_checked(&account, &mint, &authority, amount, decimals)
            .expect("Failed to create parsed instruction");

    let result = IxUtils::reconstruct_spl_token_instruction(
        &solana_parsed,
        &IxUtils::build_account_keys_hashmap(&account_keys),
    );

    assert!(result.is_ok());
    let compiled = result.unwrap();
    assert_eq!(compiled.program_id_index, 0);
    assert_eq!(compiled.accounts, vec![1, 2, 3]); // account, mint, authority indices
    assert_eq!(compiled.data, instruction.data);
}

#[test]
fn test_reconstruct_token2022_close_account_instruction() {
    let account = Pubkey::new_unique();
    let destination = Pubkey::new_unique();
    let authority = Pubkey::new_unique();
    let token_program_id = spl_token_2022_interface::ID;
    let account_keys = vec![token_program_id, account, destination, authority];

    let instruction = spl_token_2022_interface::instruction::close_account(
        &spl_token_2022_interface::ID,
        &account,
        &destination,
        &authority,
        &[],
    )
    .expect("Failed to create close_account instruction");

    let solana_parsed = create_parsed_token2022_close_account(&account, &destination, &authority)
        .expect("Failed to create parsed instruction");

    let result = IxUtils::reconstruct_spl_token_instruction(
        &solana_parsed,
        &IxUtils::build_account_keys_hashmap(&account_keys),
    );

    assert!(result.is_ok());
    let compiled = result.unwrap();
    assert_eq!(compiled.program_id_index, 0);
    assert_eq!(compiled.accounts, vec![1, 2, 3]); // account, destination, authority indices
    assert_eq!(compiled.data, instruction.data);
}

#[test]
fn test_reconstruct_token2022_approve_instruction() {
    let source = Pubkey::new_unique();
    let delegate = Pubkey::new_unique();
    let owner = Pubkey::new_unique();
    let token_program_id = spl_token_2022_interface::ID;
    let account_keys = vec![token_program_id, source, delegate, owner];
    let amount = 1200000u64;

    let instruction = spl_token_2022_interface::instruction::approve(
        &spl_token_2022_interface::ID,
        &source,
        &delegate,
        &owner,
        &[],
        amount,
    )
    .expect("Failed to create approve instruction");

    let solana_parsed = create_parsed_token2022_approve(&source, &delegate, &owner, amount)
        .expect("Failed to create parsed instruction");

    let result = IxUtils::reconstruct_spl_token_instruction(
        &solana_parsed,
        &IxUtils::build_account_keys_hashmap(&account_keys),
    );

    assert!(result.is_ok());
    let compiled = result.unwrap();
    assert_eq!(compiled.program_id_index, 0);
    assert_eq!(compiled.accounts, vec![1, 2, 3]); // source, delegate, owner indices
    assert_eq!(compiled.data, instruction.data);
}

#[test]
fn test_reconstruct_token2022_approve_checked_instruction() {
    let source = Pubkey::new_unique();
    let delegate = Pubkey::new_unique();
    let owner = Pubkey::new_unique();
    let mint = Pubkey::new_unique();
    let token_program_id = spl_token_2022_interface::ID;
    let account_keys = vec![token_program_id, source, mint, delegate, owner];
    let amount = 3500000u64;
    let decimals = 6u8;

    let instruction = spl_token_2022_interface::instruction::approve_checked(
        &spl_token_2022_interface::ID,
        &source,
        &mint,
        &delegate,
        &owner,
        &[],
        amount,
        decimals,
    )
    .expect("Failed to create approve_checked instruction");

    let solana_parsed = create_parsed_token2022_approve_checked(
        &source, &mint, &delegate, &owner, amount, decimals,
    )
    .expect("Failed to create parsed instruction");

    let result = IxUtils::reconstruct_spl_token_instruction(
        &solana_parsed,
        &IxUtils::build_account_keys_hashmap(&account_keys),
    );

    assert!(result.is_ok());
    let compiled = result.unwrap();
    assert_eq!(compiled.program_id_index, 0);
    assert_eq!(compiled.accounts, vec![1, 2, 3, 4]); // source, mint, delegate, owner indices
    assert_eq!(compiled.data, instruction.data);
}

#[test]
fn test_dispatch_routes_spl_token_via_program_id() {
    let source = Pubkey::new_unique();
    let destination = Pubkey::new_unique();
    let authority = Pubkey::new_unique();
    let token_program_id = spl_token_interface::ID;
    let mut account_keys = vec![token_program_id, source, destination, authority];
    let amount = 1000000u64;

    let parsed = create_parsed_spl_token_transfer(&source, &destination, &authority, amount)
        .expect("Failed to create parsed instruction");

    let ui_instruction = UiInstruction::Parsed(UiParsedInstruction::Parsed(parsed));

    let result = IxUtils::reconstruct_instruction_from_ui(&ui_instruction, &mut account_keys);

    assert!(result.is_ok(), "SPL token transfer should be dispatched via program_id");
    let compiled = result.unwrap();
    assert_eq!(compiled.program_id_index, 0);
    assert_eq!(compiled.accounts, vec![1, 2, 3]);
}

#[test]
fn test_dispatch_routes_token2022_via_program_id() {
    let source = Pubkey::new_unique();
    let destination = Pubkey::new_unique();
    let authority = Pubkey::new_unique();
    let token_program_id = spl_token_2022_interface::ID;
    let mut account_keys = vec![token_program_id, source, destination, authority];
    let amount = 1000000u64;

    let parsed = create_parsed_token2022_transfer(&source, &destination, &authority, amount)
        .expect("Failed to create parsed instruction");

    let ui_instruction = UiInstruction::Parsed(UiParsedInstruction::Parsed(parsed));

    let result = IxUtils::reconstruct_instruction_from_ui(&ui_instruction, &mut account_keys);

    assert!(result.is_ok(), "Token2022 transfer should be dispatched via program_id");
    let compiled = result.unwrap();
    assert_eq!(compiled.program_id_index, 0);
    assert_eq!(compiled.accounts, vec![1, 2, 3]);
}

#[test]
fn test_dispatch_routes_system_program_via_program_id() {
    let source = Pubkey::new_unique();
    let destination = Pubkey::new_unique();
    let system_program_id = SYSTEM_PROGRAM_ID;
    let mut account_keys = vec![system_program_id, source, destination];
    let lamports = 1000000u64;

    let parsed = create_parsed_system_transfer(&source, &destination, lamports)
        .expect("Failed to create parsed instruction");

    let ui_instruction = UiInstruction::Parsed(UiParsedInstruction::Parsed(parsed));

    let result = IxUtils::reconstruct_instruction_from_ui(&ui_instruction, &mut account_keys);

    assert!(result.is_ok(), "System transfer should be dispatched via program_id");
    let compiled = result.unwrap();
    assert_eq!(compiled.program_id_index, 0);
}

#[test]
fn test_reconstruct_unsupported_program_creates_stub() {
    let unsupported_program = Pubkey::new_unique();
    let mut account_keys = vec![unsupported_program];

    let parsed_instruction = solana_transaction_status_client_types::ParsedInstruction {
        program: "unsupported".to_string(),
        program_id: unsupported_program.to_string(),
        parsed: serde_json::json!({
            "type": "unknownInstruction",
            "info": {
                "someField": "someValue"
            }
        }),
        stack_height: None,
    };

    let ui_instruction = UiInstruction::Parsed(UiParsedInstruction::Parsed(parsed_instruction));

    let result = IxUtils::reconstruct_instruction_from_ui(&ui_instruction, &mut account_keys);

    assert!(result.is_ok());
    let compiled = result.unwrap();

    assert_eq!(compiled.program_id_index, 0);
    assert!(compiled.accounts.is_empty());
    assert!(compiled.data.is_empty());
}

fn assert_reconstruction_matches_original(
    label: &str,
    instruction: &Instruction,
) -> (solana_transaction_status_client_types::ParsedInstruction, Instruction) {
    let message = Message::new(std::slice::from_ref(instruction), None);
    let parsed = parse_instruction::parse(
        &instruction.program_id,
        &message.instructions[0],
        &AccountKeys::new(&message.account_keys, None),
        None,
    )
    .unwrap_or_else(|e| panic!("{label}: agave parser rejected the instruction: {e}"));
    let compiled = IxUtils::reconstruct_spl_token_instruction(
        &parsed,
        &IxUtils::build_account_keys_hashmap(&message.account_keys),
    )
    .unwrap_or_else(|e| panic!("{label}: reconstruction failed: {e}"));
    assert_eq!(compiled.accounts, message.instructions[0].accounts, "{label}: accounts");
    assert_eq!(compiled.data, message.instructions[0].data, "{label}: data");
    let reconstructed =
        IxUtils::uncompile_instructions(&[compiled], &message.account_keys).unwrap().remove(0);
    (parsed, reconstructed)
}

#[allow(deprecated)]
fn multisig_token_cases(
    program: &Pubkey,
    (source, destination, mint, authority, signers): (Pubkey, Pubkey, Pubkey, Pubkey, &[&Pubkey]),
) -> Vec<(&'static str, &'static str, Instruction)> {
    use spl_token_2022_interface::instruction as ix;
    vec![
        (
            "transfer",
            "multisigAuthority",
            ix::transfer(program, &source, &destination, &authority, signers, 5).unwrap(),
        ),
        (
            "transferChecked",
            "multisigAuthority",
            ix::transfer_checked(program, &source, &mint, &destination, &authority, signers, 5, 6)
                .unwrap(),
        ),
        (
            "burn",
            "multisigAuthority",
            ix::burn(program, &source, &mint, &authority, signers, 5).unwrap(),
        ),
        (
            "burnChecked",
            "multisigAuthority",
            ix::burn_checked(program, &source, &mint, &authority, signers, 5, 6).unwrap(),
        ),
        (
            "setAuthority",
            "multisigAuthority",
            ix::set_authority(
                program,
                &source,
                Some(&destination),
                ix::AuthorityType::AccountOwner,
                &authority,
                signers,
            )
            .unwrap(),
        ),
        (
            "approve",
            "multisigOwner",
            ix::approve(program, &source, &destination, &authority, signers, 5).unwrap(),
        ),
        (
            "approveChecked",
            "multisigOwner",
            ix::approve_checked(program, &source, &mint, &destination, &authority, signers, 5, 6)
                .unwrap(),
        ),
        ("revoke", "multisigOwner", ix::revoke(program, &source, &authority, signers).unwrap()),
        (
            "closeAccount",
            "multisigOwner",
            ix::close_account(program, &source, &destination, &authority, signers).unwrap(),
        ),
        (
            "mintTo",
            "multisigMintAuthority",
            ix::mint_to(program, &mint, &destination, &authority, signers, 5).unwrap(),
        ),
        (
            "mintToChecked",
            "multisigMintAuthority",
            ix::mint_to_checked(program, &mint, &destination, &authority, signers, 5, 6).unwrap(),
        ),
        (
            "freezeAccount",
            "multisigFreezeAuthority",
            ix::freeze_account(program, &source, &mint, &authority, signers).unwrap(),
        ),
        (
            "thawAccount",
            "multisigFreezeAuthority",
            ix::thaw_account(program, &source, &mint, &authority, signers).unwrap(),
        ),
    ]
}

#[test]
fn test_reconstruct_spl_token_multisig_authority_matches_original() {
    let (source, destination, mint, authority) =
        (Pubkey::new_unique(), Pubkey::new_unique(), Pubkey::new_unique(), Pubkey::new_unique());
    let signer_keys = [Pubkey::new_unique(), Pubkey::new_unique()];
    let signers: &[&Pubkey] = &[&signer_keys[0], &signer_keys[1]];
    let keys = (source, destination, mint, authority, signers);

    let cases = multisig_token_cases(&spl_token_interface::ID, keys)
        .into_iter()
        .chain(multisig_token_cases(&spl_token_2022_interface::ID, keys));

    for (label, multisig_field, instruction) in cases {
        let label = format!("{label} ({})", instruction.program_id);
        let (parsed, _) = assert_reconstruction_matches_original(&label, &instruction);
        let info = &parsed.parsed[PARSED_DATA_FIELD_INFO];
        assert_eq!(info[multisig_field], authority.to_string(), "{label}: {multisig_field}");
        assert_eq!(
            info[PARSED_DATA_FIELD_SIGNERS],
            serde_json::json!([signer_keys[0].to_string(), signer_keys[1].to_string()]),
            "{label}: signers"
        );
    }
}

#[test]
fn test_reconstruct_token2022_transfer_hook_transfer_checked_keeps_extra_accounts() {
    use crate::transaction::TransactionUtil;
    use solana_message::VersionedMessage;

    let source = Pubkey::new_unique();
    let mint = Pubkey::new_unique();
    let destination = Pubkey::new_unique();
    let authority = Pubkey::new_unique();
    let extra_accounts = [Pubkey::new_unique(), Pubkey::new_unique(), Pubkey::new_unique()];

    let mut instruction = spl_token_2022_interface::instruction::transfer_checked(
        &spl_token_2022_interface::ID,
        &source,
        &mint,
        &destination,
        &authority,
        &[],
        1_000,
        6,
    )
    .unwrap();
    instruction
        .accounts
        .extend(extra_accounts.iter().map(|key| AccountMeta::new_readonly(*key, false)));

    let (parsed, reconstructed) =
        assert_reconstruction_matches_original("hooked transferChecked", &instruction);
    let info = &parsed.parsed[PARSED_DATA_FIELD_INFO];
    assert!(info.get(PARSED_DATA_FIELD_AUTHORITY).is_none());
    assert_eq!(info[PARSED_DATA_FIELD_MULTISIG_AUTHORITY], authority.to_string());

    let payer = Pubkey::new_unique();
    let resolved_tx = TransactionUtil::new_unsigned_versioned_transaction_resolved(
        VersionedMessage::Legacy(Message::new(&[reconstructed], Some(&payer))),
    )
    .unwrap();

    let parsed_spl = IxUtils::parse_token_instructions(&resolved_tx).unwrap();
    match parsed_spl.get(&ParsedSPLInstructionType::SplTokenTransfer).map(Vec::as_slice) {
        Some(
            [ParsedSPLInstructionData::SplTokenTransfer { owner, multisig_signers, mint: m, .. }],
        ) => {
            assert_eq!(*owner, authority);
            assert_eq!(multisig_signers, &extra_accounts.to_vec());
            assert_eq!(*m, Some(mint));
        }
        other => panic!("expected one SplTokenTransfer, got {other:?}"),
    }
}

#[test]
fn test_reconstruct_spl_token_multisig_authority_without_signers_is_rejected() {
    let source = Pubkey::new_unique();
    let destination = Pubkey::new_unique();
    let multisig = Pubkey::new_unique();
    let account_keys = vec![spl_token_interface::ID, source, destination, multisig];

    let parsed = solana_transaction_status_client_types::ParsedInstruction {
        program: "spl-token".to_string(),
        program_id: spl_token_interface::ID.to_string(),
        parsed: serde_json::json!({
            "type": "transfer",
            "info": {
                "source": source.to_string(),
                "destination": destination.to_string(),
                "amount": "5",
                "multisigAuthority": multisig.to_string(),
            }
        }),
        stack_height: None,
    };

    let result = IxUtils::reconstruct_spl_token_instruction(
        &parsed,
        &IxUtils::build_account_keys_hashmap(&account_keys),
    );
    assert!(
        matches!(result, Err(KoraError::SerializationError(ref msg)) if msg.contains("'signers'")),
        "expected missing signers rejection, got {result:?}"
    );
}

#[test]
fn test_reconstruct_token2022_metadata_and_extension_inits_match_original() {
    use solana_nullable::MaybeNull;
    use spl_token_2022_interface::{
        extension::{
            group_member_pointer, group_pointer, metadata_pointer, transfer_fee, transfer_hook,
        },
        instruction as token_2022,
    };
    use spl_token_metadata_interface::{instruction as token_metadata, state::Field};

    let program = spl_token_2022_interface::ID;
    let mint = Pubkey::new_unique();
    let metadata = Pubkey::new_unique();
    let update_authority = Pubkey::new_unique();
    let mint_authority = Pubkey::new_unique();
    let authority = Pubkey::new_unique();
    let address = Pubkey::new_unique();
    let signer_keys = [Pubkey::new_unique(), Pubkey::new_unique()];
    let signers: &[&Pubkey] = &[&signer_keys[0], &signer_keys[1]];

    let cases = vec![
        (
            "initializeMetadataPointer",
            metadata_pointer::instruction::initialize(
                &program,
                &mint,
                Some(authority),
                Some(address),
            )
            .unwrap(),
        ),
        (
            "initializeMetadataPointer",
            metadata_pointer::instruction::initialize(&program, &mint, None, None).unwrap(),
        ),
        (
            "updateMetadataPointer",
            metadata_pointer::instruction::update(&program, &mint, &authority, &[], Some(address))
                .unwrap(),
        ),
        (
            "updateMetadataPointer",
            metadata_pointer::instruction::update(&program, &mint, &authority, signers, None)
                .unwrap(),
        ),
        (
            "initializeTokenMetadata",
            token_metadata::initialize(
                &program,
                &metadata,
                &update_authority,
                &mint,
                &mint_authority,
                "Token".to_string(),
                "TKN".to_string(),
                "https://example.com/token.json".to_string(),
            ),
        ),
        (
            "updateTokenMetadataField",
            token_metadata::update_field(
                &program,
                &metadata,
                &update_authority,
                Field::Uri,
                "https://example.com/new.json".to_string(),
            ),
        ),
        (
            "updateTokenMetadataField",
            token_metadata::update_field(
                &program,
                &metadata,
                &update_authority,
                Field::Key("website".to_string()),
                "https://example.com".to_string(),
            ),
        ),
        (
            "removeTokenMetadataKey",
            token_metadata::remove_key(
                &program,
                &metadata,
                &update_authority,
                "website".to_string(),
                true,
            ),
        ),
        (
            "updateTokenMetadataAuthority",
            token_metadata::update_authority(
                &program,
                &metadata,
                &update_authority,
                MaybeNull::try_from(Some(authority)).unwrap(),
            ),
        ),
        (
            "updateTokenMetadataAuthority",
            token_metadata::update_authority(
                &program,
                &metadata,
                &update_authority,
                MaybeNull::try_from(None).unwrap(),
            ),
        ),
        ("emitTokenMetadata", token_metadata::emit(&program, &metadata, Some(1), Some(10))),
        ("emitTokenMetadata", token_metadata::emit(&program, &metadata, None, None)),
        (
            "initializeMintCloseAuthority",
            token_2022::initialize_mint_close_authority(&program, &mint, Some(&authority)).unwrap(),
        ),
        (
            "initializeMintCloseAuthority",
            token_2022::initialize_mint_close_authority(&program, &mint, None).unwrap(),
        ),
        (
            "initializePermanentDelegate",
            token_2022::initialize_permanent_delegate(&program, &mint, &authority).unwrap(),
        ),
        (
            "initializeNonTransferableMint",
            token_2022::initialize_non_transferable_mint(&program, &mint).unwrap(),
        ),
        (
            "initializeTransferHook",
            transfer_hook::instruction::initialize(&program, &mint, Some(authority), Some(address))
                .unwrap(),
        ),
        (
            "initializeGroupPointer",
            group_pointer::instruction::initialize(&program, &mint, Some(authority), Some(address))
                .unwrap(),
        ),
        (
            "initializeGroupMemberPointer",
            group_member_pointer::instruction::initialize(
                &program,
                &mint,
                Some(authority),
                Some(address),
            )
            .unwrap(),
        ),
        (
            "initializeTransferFeeConfig",
            transfer_fee::instruction::initialize_transfer_fee_config(
                &program,
                &mint,
                Some(&authority),
                Some(&address),
                250,
                u64::MAX,
            )
            .unwrap(),
        ),
        (
            "initializeTransferFeeConfig",
            transfer_fee::instruction::initialize_transfer_fee_config(
                &program, &mint, None, None, 0, 0,
            )
            .unwrap(),
        ),
    ];

    for (index, (expected_type, instruction)) in cases.into_iter().enumerate() {
        let label = format!("case {index} ({expected_type})");
        let (parsed, _) = assert_reconstruction_matches_original(&label, &instruction);
        assert_eq!(parsed.parsed[PARSED_DATA_FIELD_TYPE], expected_type, "{label}: type");
    }
}

#[test]
fn test_reconstruct_token_metadata_custom_key_shadowing_builtin_field_keeps_accounts() {
    use spl_token_metadata_interface::{instruction as token_metadata, state::Field};

    let program = spl_token_2022_interface::ID;
    let metadata = Pubkey::new_unique();
    let update_authority = Pubkey::new_unique();
    let update_field = |field: Field| {
        token_metadata::update_field(
            &program,
            &metadata,
            &update_authority,
            field,
            "value".to_string(),
        )
    };

    for (key, builtin) in [("name", Field::Name), ("symbol", Field::Symbol), ("uri", Field::Uri)] {
        let custom_key = update_field(Field::Key(key.to_string()));
        let message = Message::new(std::slice::from_ref(&custom_key), None);
        let parsed = parse_instruction::parse(
            &program,
            &message.instructions[0],
            &AccountKeys::new(&message.account_keys, None),
            None,
        )
        .unwrap();
        assert_eq!(parsed.parsed[PARSED_DATA_FIELD_INFO][PARSED_DATA_FIELD_FIELD], key);

        let compiled = IxUtils::reconstruct_spl_token_instruction(
            &parsed,
            &IxUtils::build_account_keys_hashmap(&message.account_keys),
        )
        .unwrap();

        assert_ne!(compiled.data, custom_key.data, "{key}: Agave output is lossy");
        assert_eq!(compiled.data, update_field(builtin).data, "{key}: rebuilt as built-in field");
        assert_eq!(compiled.accounts, message.instructions[0].accounts, "{key}: accounts");
    }
}

#[test]
fn test_reconstruct_token_metadata_instruction_rejected_for_spl_token_program() {
    let metadata = Pubkey::new_unique();
    let update_authority = Pubkey::new_unique();
    let account_keys = vec![spl_token_interface::ID, metadata, update_authority];

    let parsed = solana_transaction_status_client_types::ParsedInstruction {
        program: "spl-token".to_string(),
        program_id: spl_token_interface::ID.to_string(),
        parsed: serde_json::json!({
            "type": "removeTokenMetadataKey",
            "info": {
                "metadata": metadata.to_string(),
                "updateAuthority": update_authority.to_string(),
                "key": "website",
                "idempotent": false,
            }
        }),
        stack_height: None,
    };

    let result = IxUtils::reconstruct_spl_token_instruction(
        &parsed,
        &IxUtils::build_account_keys_hashmap(&account_keys),
    );
    assert!(
        matches!(result, Err(KoraError::InvalidTransaction(ref msg))
            if msg.contains("Unrecognized SPL Token instruction type 'removeTokenMetadataKey'")),
        "expected spl-token metadata CPI rejection, got {result:?}"
    );
}

fn reconstruct_parsed_ata(instruction: Instruction) -> (CompiledInstruction, CompiledInstruction) {
    let ata_program = spl_associated_token_account_interface::program::id();
    let message = Message::new(&[instruction], None);
    let compiled = message.instructions[0].clone();
    let parsed = parse_instruction::parse(
        &ata_program,
        &compiled,
        &AccountKeys::new(&message.account_keys, None),
        None,
    )
    .unwrap();

    let mut account_keys = message.account_keys.clone();
    let reconstructed = IxUtils::reconstruct_instruction_from_ui(
        &UiInstruction::Parsed(UiParsedInstruction::Parsed(parsed)),
        &mut account_keys,
    )
    .unwrap();
    (compiled, reconstructed)
}

#[test]
fn test_reconstruct_parsed_ata_create_matches_compiled() {
    let (compiled, reconstructed) = reconstruct_parsed_ata(
        spl_associated_token_account_interface::instruction::create_associated_token_account(
            &Pubkey::new_unique(),
            &Pubkey::new_unique(),
            &Pubkey::new_unique(),
            &spl_token_interface::id(),
        ),
    );

    assert_eq!(reconstructed, compiled);
    assert_eq!(reconstructed.data, vec![0]);
}

#[test]
fn test_reconstruct_parsed_ata_create_idempotent_matches_compiled() {
    let (compiled, reconstructed) = reconstruct_parsed_ata(
        spl_associated_token_account_interface::instruction::create_associated_token_account_idempotent(
            &Pubkey::new_unique(),
            &Pubkey::new_unique(),
            &Pubkey::new_unique(),
            &spl_token_2022_interface::id(),
        ),
    );

    assert_eq!(reconstructed, compiled);
    assert_eq!(reconstructed.data, vec![1]);
}
