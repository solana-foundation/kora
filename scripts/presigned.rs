use solana_client::rpc_client::RpcClient;
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

    // Create RPC client for devnet
    let rpc_client = RpcClient::new("https://api.devnet.solana.com".to_string());

    let instruction = system_instruction::transfer(&sender.pubkey(), &recipient, amount);

    // Get recent blockhash from devnet
    let recent_blockhash = rpc_client.get_latest_blockhash().unwrap();

    let message =
        Message::new_with_blockhash(&[instruction], Some(&sender.pubkey()), &recent_blockhash);

    let transaction = Transaction { signatures: vec![Default::default()], message };

    let serialized = bincode::serialize(&transaction).unwrap();
    let base58_tx = bs58::encode(serialized).into_string();

    println!("Sender pubkey: {}", sender.pubkey());
    println!("Base58 encoded unsigned transaction: {}", base58_tx);
}
