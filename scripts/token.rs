use solana_client::rpc_client::RpcClient;
use solana_sdk::{
    commitment_config::CommitmentConfig,
    program_pack::Pack,
    pubkey::Pubkey,
    signature::{Keypair, Signer},
    system_instruction,
    transaction::Transaction,
};
use spl_token::{
    instruction as token_instruction, program::TokenProgram, state::Account as TokenAccount,
    type_::TokenType,
};
use std::str::FromStr;

fn main() {
    // Define token_interface as an instance of a struct implementing TokenInterface
    let token_interface = TokenProgram::new(TokenType::Spl);

    // Use token_interface to get the program ID
    let program_id = token_interface.program_id();

    // Connect to Solana cluster
    let rpc_url = "https://api.devnet.solana.com".to_string(); // Change to mainnet for production
    let client = RpcClient::new_with_commitment(rpc_url, CommitmentConfig::confirmed());

    // USDC mint address (this is devnet USDC, replace with mainnet USDC for production)
    let usdc_mint = Pubkey::from_str("4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU").unwrap();

    // Your wallet keypair (load this from a file in production)
    let payer = Keypair::new();

    // Generate a new token account
    let token_account = Keypair::new();

    // Calculate minimum rent for token account
    let rent = client.get_minimum_balance_for_rent_exemption(TokenAccount::LEN).unwrap();

    // Create token account
    let create_account_ix = system_instruction::create_account(
        &payer.pubkey(),
        &token_account.pubkey(),
        rent,
        TokenAccount::LEN as u64,
        &program_id,
    );

    // Initialize token account
    let init_account_ix = token_instruction::initialize_account(
        &program_id,
        &token_account.pubkey(),
        &usdc_mint,
        &payer.pubkey(),
    )
    .unwrap();

    // Set close authority instruction
    let set_authority_ix = token_instruction::set_authority(
        &program_id,
        &token_account.pubkey(),
        Some(&payer.pubkey()),
        spl_token::instruction::AuthorityType::CloseAccount,
        &payer.pubkey(),
        &[],
    )
    .unwrap();

    // Create transaction
    let recent_blockhash = client.get_latest_blockhash().unwrap();
    let transaction = Transaction::new_signed_with_payer(
        &[create_account_ix, init_account_ix, set_authority_ix],
        Some(&payer.pubkey()),
        &[&payer, &token_account],
        recent_blockhash,
    );

    // Send and confirm transaction
    match client.send_and_confirm_transaction(&transaction) {
        Ok(signature) => {
            println!("Token account created successfully!");
            println!("Transaction signature: {}", signature);
            println!("Token account address: {}", token_account.pubkey());
        }
        Err(e) => println!("Error creating token account: {}", e),
    }
}
