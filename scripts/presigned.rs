use kora_lib::transaction::{encode_b64_transaction, new_unsigned_versioned_transaction};
use solana_client::rpc_client::RpcClient;
use solana_message::{Message, VersionedMessage};
use solana_sdk::{
    pubkey::Pubkey,
    signature::{Keypair, Signer},
};
use solana_system_interface::instruction::transfer;
use std::str::FromStr;

fn main() {
    let sender = Keypair::new();
    let recipient = Pubkey::from_str("AVmDft8deQEo78bRKcGN5ZMf3hyjeLBK4Rd4xGB46yQM").unwrap();
    let amount = 100;

    // Create RPC client for devnet
    let rpc_client = RpcClient::new("https://api.devnet.solana.com".to_string());

    let instruction = transfer(&sender.pubkey(), &recipient, amount);

    // Get recent blockhash from devnet
    let recent_blockhash = rpc_client.get_latest_blockhash().unwrap();

    let message = VersionedMessage::Legacy(Message::new_with_blockhash(
        &[instruction],
        Some(&sender.pubkey()),
        &recent_blockhash,
    ));

    let mut transaction = new_unsigned_versioned_transaction(message);
    transaction.signatures = vec![Default::default()];

    let base64_tx = encode_b64_transaction(&transaction).unwrap();

    println!("Sender pubkey: {}", sender.pubkey());
    println!("Base64 encoded unsigned transaction: {base64_tx}");
}
