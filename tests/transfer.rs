#[test]
fn test_create_transfer_transaction() {
    let rpc = RpcClient::new("https://api.devnet.solana.com".to_string());
    let from_keypair = Keypair::new();
    let to = Pubkey::new_unique();

    let result = create_transfer_transaction(&rpc, &from_keypair, &to, 1000, None);
    assert!(result.is_ok());
}
