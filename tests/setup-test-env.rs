use testing_utils::{check_test_validator, setup_test_accounts};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🔧 Setting up test environment...");

    // Check if test validator is running
    if !check_test_validator().await {
        eprintln!("❌ Error: Solana test validator is not running");
        eprintln!("Please start it with: surfpool start or solana-test-validator");
        std::process::exit(1);
    }

    println!("✅ Test validator is running");

    // Setup test accounts
    match setup_test_accounts().await {
        Ok(account_info) => {
            println!("✅ Test environment setup complete!");
            println!();
            println!("📋 Account Summary:");
            println!("  Fee Payer: {}", account_info.fee_payer_pubkey);
            println!("  Sender: {}", account_info.sender_pubkey);
            println!("  Recipient: {}", account_info.recipient_pubkey);
            println!("  USDC Mint: {}", account_info.usdc_mint_pubkey);
            println!("  Sender Token Account: {}", account_info.sender_token_account);
            println!("  Recipient Token Account: {}", account_info.recipient_token_account);
            println!();
            println!("🎯 Ready to run integration tests!");
            println!("Run: cargo test --test integration");
        }
        Err(e) => {
            eprintln!("❌ Failed to setup test environment: {}", e);
            std::process::exit(1);
        }
    }

    Ok(())
}
