use kora_lib::token::{TokenProgram, TokenType, TokenInterface};
use solana_client::rpc_client::RpcClient;
use solana_sdk::pubkey::Pubkey;
use spl_token_2022;

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
    let transfer_ix = program.create_transfer_instruction(
        &ata,
        &destination,
        &owner,
        1000,
    );
    assert!(transfer_ix.is_ok());
}

#[tokio::test]
async fn test_token2022_operations() {
    let program = TokenProgram::new(TokenType::Token2022);
    
    // Verify program ID matches Token2022
    assert_eq!(program.program_id(), spl_token_2022::id());

    // Test ATA derivation
    let mint = Pubkey::new_unique();
    let owner = Pubkey::new_unique();
    let ata = program.get_associated_token_address(&owner, &mint);
    assert_ne!(ata, owner);
    assert_ne!(ata, mint);

    // Test transfer instruction creation
    let destination = Pubkey::new_unique();
    let transfer_ix = program.create_transfer_instruction(
        &ata,
        &destination,
        &owner,
        1000,
    );
    assert!(transfer_ix.is_ok());

    // Test transfer checked instruction
    let transfer_checked_ix = program.create_transfer_checked_instruction(
        &ata,
        &mint,
        &destination,
        &owner,
        1000,
        9, // Standard token decimals
    );
    assert!(transfer_checked_ix.is_ok());

    // Test ATA creation instruction
    let funder = Pubkey::new_unique();
    let create_ata_ix = program.create_associated_token_account_instruction(
        &funder,
        &owner,
        &mint,
    );
    assert_eq!(create_ata_ix.program_id, spl_associated_token_account::id());
}

#[test]
fn test_token2022_program_id() {
    let program = TokenProgram::new(TokenType::Token2022);
    assert_eq!(program.program_id(), spl_token_2022::id());
} 