use solana_message::{
    compiled_instruction::CompiledInstruction, v0, v1, Message, MessageHeader, VersionedMessage,
};
use solana_sdk::{
    hash::Hash,
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    signature::Keypair,
    signer::Signer,
    transaction::{Transaction, VersionedTransaction},
};
use solana_system_interface::instruction::transfer;

use crate::transaction::{TransactionUtil, VersionedTransactionResolved};

pub fn create_mock_encoded_transaction() -> String {
    let ix = transfer(&Pubkey::new_unique(), &Pubkey::new_unique(), 1000000000);
    let message = VersionedMessage::Legacy(Message::new(&[ix], Some(&Pubkey::new_unique())));
    let transaction = TransactionUtil::new_unsigned_versioned_transaction(message);

    TransactionUtil::encode_versioned_transaction(&transaction).unwrap()
}

pub fn create_mock_transaction() -> VersionedTransaction {
    let keypair = Keypair::new();
    let instruction = transfer(&keypair.pubkey(), &Pubkey::new_unique(), 1000);
    let message = Message::new(&[instruction], Some(&keypair.pubkey()));
    let transaction = Transaction::new_unsigned(message);
    VersionedTransaction::from(transaction)
}

pub fn create_mock_resolved_transaction() -> VersionedTransactionResolved {
    let tx = create_mock_transaction();
    VersionedTransactionResolved::from_kora_built_transaction(&tx).unwrap()
}

/// Sign a V1 transaction carrying `config`, from fixed key material so a failure
/// reproduces from the test alone.
pub fn create_mock_v1_transaction(config: v1::TransactionConfig) -> VersionedTransaction {
    let keypair = Keypair::new_from_array([3; 32]);
    let instruction = Instruction::new_with_bytes(
        Pubkey::new_from_array([9; 32]),
        &[1, 2, 3],
        vec![AccountMeta::new(keypair.pubkey(), true)],
    );
    let message = VersionedMessage::V1(
        v1::Message::try_compile_with_config(
            &keypair.pubkey(),
            &[instruction],
            Hash::new_from_array([5; 32]),
            config,
        )
        .unwrap(),
    );

    VersionedTransaction::try_new(message, &[&keypair]).unwrap()
}

pub fn create_mock_encoded_v1_transaction(config: v1::TransactionConfig) -> String {
    TransactionUtil::encode_versioned_transaction(&create_mock_v1_transaction(config)).unwrap()
}

/// Sign a V1 transaction whose account list already sits at the format's 64-address limit,
/// so appending any account the message does not already hold makes it unsanitary.
pub fn create_mock_v1_transaction_with_max_addresses(fee_payer: &Keypair) -> VersionedTransaction {
    let mut account_keys = vec![fee_payer.pubkey()];
    account_keys.extend((1..v1::MAX_ADDRESSES).map(|index| Pubkey::new_from_array([index; 32])));

    let message = VersionedMessage::V1(v1::Message {
        header: MessageHeader {
            num_required_signatures: 1,
            num_readonly_signed_accounts: 0,
            num_readonly_unsigned_accounts: v1::MAX_ADDRESSES - 1,
        },
        config: v1::TransactionConfig::empty(),
        lifetime_specifier: Hash::new_from_array([7; 32]),
        account_keys,
        instructions: vec![CompiledInstruction {
            program_id_index: 1,
            accounts: vec![0],
            data: vec![1, 2, 3],
        }],
    });

    VersionedTransaction::try_new(message, &[fee_payer]).unwrap()
}

pub fn create_legacy_message(fee_payer: &Pubkey, instructions: &[Instruction]) -> VersionedMessage {
    VersionedMessage::Legacy(Message::new(instructions, Some(fee_payer)))
}

/// V1 message carrying a transfer instruction and the given transaction config.
pub fn create_v1_message(fee_payer: &Pubkey, config: v1::TransactionConfig) -> VersionedMessage {
    create_v1_message_with_instructions(
        fee_payer,
        &[transfer(fee_payer, &Pubkey::new_unique(), 1_000)],
        config,
    )
}

/// V1 message carrying the given instructions and transaction config.
pub fn create_v1_message_with_instructions(
    fee_payer: &Pubkey,
    instructions: &[Instruction],
    config: v1::TransactionConfig,
) -> VersionedMessage {
    let mut message =
        v1::Message::try_compile(fee_payer, instructions, Hash::new_unique()).unwrap();
    message.config = config;
    VersionedMessage::V1(message)
}

/// V0 message whose instructions all invoke a program loaded from a lookup table:
/// `program_id_index` 1 resolves past the single static key, so callers must supply
/// the resolved key list themselves (see [`create_resolved_with_loaded_keys`]).
pub fn create_v0_message_with_alt_loaded_program(
    fee_payer: &Pubkey,
    instruction_data: Vec<Vec<u8>>,
) -> VersionedMessage {
    VersionedMessage::V0(v0::Message {
        header: MessageHeader {
            num_required_signatures: 1,
            num_readonly_signed_accounts: 0,
            num_readonly_unsigned_accounts: 0,
        },
        account_keys: vec![*fee_payer],
        recent_blockhash: Hash::new_unique(),
        instructions: instruction_data
            .into_iter()
            .map(|data| CompiledInstruction { program_id_index: 1, accounts: vec![], data })
            .collect(),
        address_table_lookups: vec![v0::MessageAddressTableLookup {
            account_key: Pubkey::new_unique(),
            writable_indexes: vec![],
            readonly_indexes: vec![0],
        }],
    })
}

/// Resolved transaction with an explicit resolved key list, for messages whose
/// program ids live in a lookup table: `from_kora_built_transaction` cannot resolve
/// those without RPC. `all_instructions` is left empty, so this only suits
/// validations that read the message and resolved keys.
pub fn create_resolved_with_loaded_keys(
    message: VersionedMessage,
    all_account_keys: Vec<Pubkey>,
) -> VersionedTransactionResolved {
    let mut resolved = create_mock_resolved_transaction();
    resolved.transaction = TransactionUtil::new_unsigned_versioned_transaction(message);
    resolved.all_account_keys = all_account_keys;
    resolved.all_instructions = Vec::new();
    resolved
}
