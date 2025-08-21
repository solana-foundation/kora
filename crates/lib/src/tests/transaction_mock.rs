use solana_message::{Message, VersionedMessage};
use solana_sdk::{pubkey::Pubkey, system_instruction::transfer};

use crate::transaction::TransactionUtil;

pub fn create_mock_transaction() -> String {
    let ix = transfer(&Pubkey::new_unique(), &Pubkey::new_unique(), 1000000000);
    let message = VersionedMessage::Legacy(Message::new(&[ix], Some(&Pubkey::new_unique())));
    let transaction = TransactionUtil::new_unsigned_versioned_transaction(message);

    TransactionUtil::encode_versioned_transaction(&transaction)
}
