use kora_lib::token::{TokenInterface, TokenProgram, TokenType};
use solana_client::rpc_client::RpcClient;
use solana_sdk::pubkey::Pubkey;

#[tokio::test]
async fn test_token_operations() {
    let rpc_url = "https://api.devnet.solana.com".to_string();
    let client = RpcClient::new(rpc_url);
    let program = TokenProgram::new(TokenType::Spl);

    // Test with known devnet SPL token
    let mint = Pubkey::new_unique();
    let owner = Pubkey::new_unique();
    let destination = Pubkey::new_unique();

    // Test ATA derivation
    let ata = program.get_associated_token_address(&owner, &mint);
    assert_ne!(ata, owner);
    assert_ne!(ata, mint);

    // Test transfer instruction creation
    let transfer_ix = program.create_transfer_instruction(&ata, &destination, &owner, 1000);
    assert!(transfer_ix.is_ok());
}
