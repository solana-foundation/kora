use solana_sdk::{
    message::Message,
    pubkey::Pubkey,
    signature::{Keypair, Signer},
    system_instruction,
    transaction::Transaction,
};
use std::str::FromStr;

fn main() {
    let sender = Keypair::new();

    let recipient = Pubkey::from_str("AVmDft8deQEo78bRKcGN5ZMf3hyjeLBK4Rd4xGB46yQM").unwrap();

    let amount = 100;

    let instruction = system_instruction::transfer(&sender.pubkey(), &recipient, amount);

    let message = Message::new(
        &[instruction],
        Some(&sender.pubkey()), // Fee payer
    );

    let mut transaction = Transaction::new_unsigned(message);
    transaction.sign(&[&sender], transaction.message.recent_blockhash);

    let serialized = bincode::serialize(&transaction).unwrap();
    let base58_tx = bs58::encode(serialized).into_string();

    println!("Sender pubkey: {}", sender.pubkey());
    println!("Sender public key bytes: {:?}", sender.pubkey().to_bytes());
    println!("Sender private key (32 bytes): {:?}", sender.secret());
    println!("Full keypair bytes (64 bytes): {:?}", sender.to_bytes());
    println!("Full private key (64 bytes): {:?}", bs58::encode(sender.to_bytes()).into_string());
    println!("Base58 encoded transaction: {}", base58_tx);
}
