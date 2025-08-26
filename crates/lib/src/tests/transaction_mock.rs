use solana_message::{Message, VersionedMessage};
use solana_sdk::{
    pubkey::Pubkey,
    signature::Keypair,
    signer::Signer,
    system_instruction::transfer,
    transaction::{Transaction, VersionedTransaction},
};

use crate::transaction::TransactionUtil;

pub fn create_mock_encoded_transaction() -> String {
    let ix = transfer(&Pubkey::new_unique(), &Pubkey::new_unique(), 1000000000);
    let message = VersionedMessage::Legacy(Message::new(&[ix], Some(&Pubkey::new_unique())));
    let transaction = TransactionUtil::new_unsigned_versioned_transaction(message);

    TransactionUtil::encode_versioned_transaction(&transaction)
}

pub fn create_mock_transaction() -> VersionedTransaction {
    let keypair = Keypair::new();
    let instruction = transfer(&keypair.pubkey(), &Pubkey::new_unique(), 1000);
    let message = Message::new(&[instruction], Some(&keypair.pubkey()));
    let transaction = Transaction::new_unsigned(message);
    VersionedTransaction::from(transaction)
}
