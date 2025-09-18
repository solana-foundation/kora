use base64::{engine::general_purpose::STANDARD, Engine};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::pubkey::Pubkey;
use std::{fs, path::Path, sync::Arc};
use tests::common::{
    constants::DEFAULT_RPC_URL, setup::TestAccountSetup, TestAccountInfo, KORA_PRIVATE_KEY_ENV,
    PAYMENT_ADDRESS_KEYPAIR_ENV, SIGNER_2_KEYPAIR_ENV, TEST_ALLOWED_LOOKUP_TABLE_ADDRESS_ENV,
    TEST_DISALLOWED_LOOKUP_TABLE_ADDRESS_ENV, TEST_INTEREST_BEARING_MINT_KEYPAIR_ENV,
    TEST_RECIPIENT_PUBKEY_ENV, TEST_SENDER_KEYPAIR_ENV, TEST_SERVER_URL_ENV,
    TEST_TRANSACTION_LOOKUP_TABLE_ADDRESS_ENV, TEST_TRANSFER_HOOK_MINT_KEYPAIR_ENV,
    TEST_USDC_MINT_2022_KEYPAIR_ENV, TEST_USDC_MINT_KEYPAIR_ENV,
};
use tokio::{process::Child, task::JoinSet};

const TEST_ACCOUNTS_DIR: &str = "tests/src/common/fixtures/test-accounts";
const TRANSFER_HOOK_PROGRAM_ID: &str = "Bcdikjss8HWzKEuj6gEQoFq9TCnGnk6v3kUnRU1gb6hA";
const TRANSFER_HOOK_PROGRAM_PATH: &str =
    "tests/src/common/transfer-hook-example/transfer_hook_example.so";

#[derive(Debug, Clone, Copy)]
enum AccountFile {
    FeePayer,
    Sender,
    Recipient,
    UsdcMint,
    SenderTokenAccount,
    RecipientTokenAccount,
    FeePayerTokenAccount,
    UsdcMint2022,
    SenderToken2022Account,
    RecipientToken2022Account,
    FeePayerToken2022Account,
    AllowedLookupTable,
    DisallowedLookupTable,
    TransactionLookupTable,
    Signer2,
    InterestBearingMint,
    TransferHookMint,
    Payment,
}

impl AccountFile {
    fn filename(&self) -> &'static str {
        match self {
            Self::FeePayer => "fee-payer-local.json",
            Self::Sender => "sender-local.json",
            Self::Recipient => "recipient-local.json",
            Self::UsdcMint => "usdc-mint-local.json",
            Self::SenderTokenAccount => "sender-token-account-local.json",
            Self::RecipientTokenAccount => "recipient-token-account-local.json",
            Self::FeePayerTokenAccount => "fee-payer-token-account-local.json",
            Self::UsdcMint2022 => "usdc-mint-2022-local.json",
            Self::SenderToken2022Account => "sender-token-2022-account-local.json",
            Self::RecipientToken2022Account => "recipient-token-2022-account-local.json",
            Self::FeePayerToken2022Account => "fee-payer-token-2022-account-local.json",
            Self::AllowedLookupTable => "allowed-lookup-table-local.json",
            Self::DisallowedLookupTable => "disallowed-lookup-table-local.json",
            Self::TransactionLookupTable => "transaction-lookup-table-local.json",
            Self::Signer2 => "signer2-local.json",
            Self::InterestBearingMint => "mint-2022-interest-bearing.json",
            Self::TransferHookMint => "mint-transfer-hook-local.json",
            Self::Payment => "payment-local.json",
        }
    }

    fn local_key_env_var(&self) -> &'static str {
        match self {
            Self::FeePayer => KORA_PRIVATE_KEY_ENV,
            Self::Sender => TEST_SENDER_KEYPAIR_ENV,
            Self::Recipient => TEST_RECIPIENT_PUBKEY_ENV,
            Self::UsdcMint => TEST_USDC_MINT_KEYPAIR_ENV,
            Self::UsdcMint2022 => TEST_USDC_MINT_2022_KEYPAIR_ENV,
            Self::AllowedLookupTable => TEST_ALLOWED_LOOKUP_TABLE_ADDRESS_ENV,
            Self::DisallowedLookupTable => TEST_DISALLOWED_LOOKUP_TABLE_ADDRESS_ENV,
            Self::TransactionLookupTable => TEST_TRANSACTION_LOOKUP_TABLE_ADDRESS_ENV,
            Self::Signer2 => SIGNER_2_KEYPAIR_ENV,
            Self::InterestBearingMint => TEST_INTEREST_BEARING_MINT_KEYPAIR_ENV,
            Self::TransferHookMint => TEST_TRANSFER_HOOK_MINT_KEYPAIR_ENV,
            Self::Payment => PAYMENT_ADDRESS_KEYPAIR_ENV,
            _ => panic!("Invalid account env"),
        }
    }

    fn local_key_path(&self) -> String {
        format!("tests/src/common/local-keys/{}", self.filename())
    }

    fn test_account_path(&self) -> std::path::PathBuf {
        Path::new(TEST_ACCOUNTS_DIR).join(self.filename())
    }

    fn required_test_accounts() -> &'static [AccountFile] {
        &[
            Self::FeePayer,
            Self::Sender,
            Self::Recipient,
            Self::UsdcMint,
            Self::SenderTokenAccount,
            Self::RecipientTokenAccount,
            Self::FeePayerTokenAccount,
            Self::UsdcMint2022,
            Self::SenderToken2022Account,
            Self::RecipientToken2022Account,
            Self::FeePayerToken2022Account,
            Self::AllowedLookupTable,
            Self::DisallowedLookupTable,
            Self::TransactionLookupTable,
            Self::Signer2,
            Self::InterestBearingMint,
            Self::TransferHookMint,
            Self::Payment,
        ]
    }

    fn required_test_accounts_env_vars() -> &'static [AccountFile] {
        &[
            Self::FeePayer,
            Self::Signer2,
            Self::Sender,
            Self::UsdcMint,
            Self::UsdcMint2022,
            Self::InterestBearingMint,
            Self::TransferHookMint,
            Self::Payment,
        ]
    }

    fn set_environment_variable(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        std::env::set_var(self.local_key_env_var(), fs::read_to_string(self.local_key_path())?);
        Ok(())
    }

    fn set_dynamic_environment_variable(
        &self,
        value: &str,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        std::env::set_var(self.local_key_env_var(), value);
        Ok(())
    }

    async fn save_account_for_file(
        &self,
        client: &RpcClient,
        address: &Pubkey,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        save_account(client, address, self.test_account_path()).await
    }

    fn get_as_env_var(&self) -> (&'static str, String) {
        (self.local_key_env_var(), std::env::var(self.local_key_env_var()).unwrap())
    }
}

pub struct TestRunner {
    pub rpc_client: Arc<RpcClient>,
    pub reqwest_client: reqwest::Client,
    pub solana_test_validator_pid: Option<Child>,
    pub test_accounts: TestAccountInfo,
    pub kora_pids: Vec<Child>,
}

/*
Local validator and test environment setup
*/

pub async fn check_test_validator() -> bool {
    let client = RpcClient::new(DEFAULT_RPC_URL.to_string());
    client.get_health().await.is_ok()
}

pub async fn start_test_validator(
    load_accounts: bool,
) -> Result<Child, Box<dyn std::error::Error + Send + Sync>> {
    let mut cmd = tokio::process::Command::new("solana-test-validator");
    cmd.arg("--reset").arg("--quiet");

    if Path::new(TRANSFER_HOOK_PROGRAM_PATH).exists() {
        cmd.arg("--bpf-program").arg(TRANSFER_HOOK_PROGRAM_ID).arg(TRANSFER_HOOK_PROGRAM_PATH);
    } else {
        println!("⚠️  Transfer hook program not found at: {TRANSFER_HOOK_PROGRAM_PATH}");
        println!("   Starting validator without transfer hook program");
        println!("   Run 'make build-transfer-hook' to build it if needed");
    }

    if load_accounts {
        for account_file in AccountFile::required_test_accounts() {
            let account_path = account_file.test_account_path();
            if account_path.exists() {
                if let Ok(account_address) = get_account_address_from_file(&account_path) {
                    cmd.arg("--account").arg(&account_address).arg(&account_path);
                    println!(
                        "Loading account: {} from {}",
                        account_address,
                        account_path.display()
                    );
                }
            }
        }
    }

    let validator_pid =
        cmd.stdout(std::process::Stdio::piped()).stderr(std::process::Stdio::piped()).spawn()?;

    let mut attempts = 0;
    while !check_test_validator().await {
        attempts += 1;
        if attempts > 30 {
            return Err("Solana test validator failed to start within 60 seconds".into());
        }
        tokio::time::sleep(std::time::Duration::from_secs(2)).await;
    }

    println!("Solana test validator started successfully");
    Ok(validator_pid)
}

pub fn set_environment_variables() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    for account_file in AccountFile::required_test_accounts_env_vars() {
        account_file.set_environment_variable()?;
    }

    Ok(())
}

pub async fn set_lookup_table_environment_variables(
    test_accounts: &TestAccountInfo,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    AccountFile::AllowedLookupTable
        .set_dynamic_environment_variable(&test_accounts.allowed_lookup_table.to_string())?;
    AccountFile::DisallowedLookupTable
        .set_dynamic_environment_variable(&test_accounts.disallowed_lookup_table.to_string())?;
    AccountFile::TransactionLookupTable
        .set_dynamic_environment_variable(&test_accounts.transaction_lookup_table.to_string())?;
    Ok(())
}

pub fn get_account_address_from_file(
    account_path: &Path,
) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    let account_json = fs::read_to_string(account_path)?;
    let account_data: serde_json::Value = serde_json::from_str(&account_json)?;

    if let Some(pubkey) = account_data["account"]["pubkey"].as_str() {
        return Ok(pubkey.to_string());
    }

    if let Some(pubkey) = account_data["pubkey"].as_str() {
        return Ok(pubkey.to_string());
    }

    Err("Could not find pubkey in account file".into())
}

pub async fn setup_test_env_from_scratch(
) -> Result<TestAccountInfo, Box<dyn std::error::Error + Send + Sync>> {
    let mut setup = TestAccountSetup::new().await;
    let test_accounts = setup.setup_all_accounts().await?;

    Ok(test_accounts)
}

pub async fn download_accounts(
    test_runner: &TestRunner,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let accounts_dir = Path::new(TEST_ACCOUNTS_DIR);
    fs::create_dir_all(accounts_dir)?;

    let client = &test_runner.rpc_client;
    let test_accounts = &test_runner.test_accounts;
    AccountFile::FeePayer.save_account_for_file(client, &test_accounts.fee_payer_pubkey).await?;
    AccountFile::Sender.save_account_for_file(client, &test_accounts.sender_pubkey).await?;
    AccountFile::Recipient.save_account_for_file(client, &test_accounts.recipient_pubkey).await?;
    AccountFile::UsdcMint.save_account_for_file(client, &test_accounts.usdc_mint_pubkey).await?;
    AccountFile::SenderTokenAccount
        .save_account_for_file(client, &test_accounts.sender_token_account)
        .await?;
    AccountFile::RecipientTokenAccount
        .save_account_for_file(client, &test_accounts.recipient_token_account)
        .await?;
    AccountFile::FeePayerTokenAccount
        .save_account_for_file(client, &test_accounts.fee_payer_token_account)
        .await?;
    AccountFile::UsdcMint2022
        .save_account_for_file(client, &test_accounts.usdc_mint_2022_pubkey)
        .await?;
    AccountFile::SenderToken2022Account
        .save_account_for_file(client, &test_accounts.sender_token_2022_account)
        .await?;
    AccountFile::RecipientToken2022Account
        .save_account_for_file(client, &test_accounts.recipient_token_2022_account)
        .await?;
    AccountFile::FeePayerToken2022Account
        .save_account_for_file(client, &test_accounts.fee_payer_token_2022_account)
        .await?;
    AccountFile::AllowedLookupTable
        .save_account_for_file(client, &test_accounts.allowed_lookup_table)
        .await?;
    AccountFile::DisallowedLookupTable
        .save_account_for_file(client, &test_accounts.disallowed_lookup_table)
        .await?;
    AccountFile::TransactionLookupTable
        .save_account_for_file(client, &test_accounts.transaction_lookup_table)
        .await?;
    Ok(())
}

async fn save_account(
    client: &RpcClient,
    address: &Pubkey,
    path: std::path::PathBuf,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let account = client.get_account(address).await?;

    let account_data = serde_json::json!({
        "pubkey": address.to_string(),
        "account": {
            "lamports": account.lamports,
            "data": [STANDARD.encode(&account.data), "base64"],
            "owner": account.owner.to_string(),
            "executable": account.executable,
            "rentEpoch": 0
        }
    });

    std::fs::write(path, serde_json::to_string_pretty(&account_data)?)?;

    Ok(())
}

async fn setup_test_env(
    test_runner: &mut TestRunner,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mut found_all_accounts = true;

    for account_file in AccountFile::required_test_accounts() {
        if !account_file.test_account_path().exists() {
            found_all_accounts = false;
            break;
        }
    }

    test_runner.solana_test_validator_pid = Some(start_test_validator(found_all_accounts).await?);

    set_environment_variables()?;

    test_runner.test_accounts = setup_test_env_from_scratch().await?;

    if !found_all_accounts {
        download_accounts(test_runner).await?;
    }
    set_lookup_table_environment_variables(&test_runner.test_accounts).await?;

    Ok(())
}

/*
Kora RPC server setup
*/
pub async fn is_kora_running(port: &str) -> bool {
    let client = reqwest::Client::new();
    let url = format!("http://127.0.0.1:{port}/liveness");
    client.get(&url).send().await.is_ok()
}

pub async fn start_kora_rpc_server(
    rpc_url: String,
    config_file: &str,
    signers_config: &str,
    port: &str,
) -> Result<Child, Box<dyn std::error::Error + Send + Sync>> {
    let fee_payer_key = fs::read_to_string(AccountFile::FeePayer.local_key_path())?;
    let signer_2 = fs::read_to_string(AccountFile::Signer2.local_key_path())?;

    let kora_pid = tokio::process::Command::new("cargo")
        .args([
            "run",
            "-p",
            "kora-cli",
            "--bin",
            "kora",
            "--",
            "--config",
            config_file,
            "--rpc-url",
            rpc_url.as_str(),
            "rpc",
            "start",
            "--signers-config",
            signers_config,
            "--port",
            port,
        ])
        .env("KORA_PRIVATE_KEY", fee_payer_key.trim())
        .env("KORA_PRIVATE_KEY_2", signer_2.trim())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()?;

    Ok(kora_pid)
}

pub async fn run_test_phase(
    phase_name: &str,
    rpc_url: String,
    config_file: &str,
    signers_config: &str,
    port: &str,
    test_names: Vec<&str>,
    initialize_payment_atas: bool,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let color = TestPhaseColor::from_port(port);
    println!("{}", color.colorize(&format!("=== Starting {phase_name} ===")));

    let mut kora_pid =
        start_kora_rpc_server(rpc_url.clone(), config_file, signers_config, port).await?;

    let mut attempts = 0;
    while !is_kora_running(port).await {
        attempts += 1;
        if attempts > 30 {
            return Err(
                format!("Kora server failed to start on port {port} within 30 seconds").into()
            );
        }
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
    }
    println!("{}", color.colorize(&format!("Kora server started on port {port}")));

    let result = async {
        if initialize_payment_atas {
            run_initialize_atas_for_kora_cli_tests(config_file, &rpc_url, signers_config, color)
                .await?;
        }

        for test_name in test_names {
            println!("{}", color.colorize(&format!("Running {test_name} tests on port {port}")));
            run_tests(port, test_name, color).await?;
        }

        Ok::<(), Box<dyn std::error::Error + Send + Sync>>(())
    }
    .await;

    kora_pid.kill().await.ok();

    match &result {
        Ok(_) => println!("{}", color.colorize(&format!("=== Completed {phase_name} ==="))),
        Err(e) => {
            println!("{}", color.colorize(&format!("=== Failed {phase_name} - Error: {e} ===")))
        }
    }

    result
}

pub async fn run_all_test_phases(
    test_runner: &TestRunner,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let rpc_url = test_runner.rpc_client.url();

    let mut join_set = JoinSet::new();

    join_set.spawn({
        let rpc_url = rpc_url.clone();
        async move {
            run_test_phase(
                "Regular Integration Tests",
                rpc_url,
                "tests/src/common/fixtures/kora-test.toml",
                "tests/src/common/fixtures/signers.toml",
                "8080",
                vec!["rpc", "tokens", "external"],
                false,
            )
            .await
        }
    });

    join_set.spawn({
        let rpc_url = rpc_url.clone();
        async move {
            run_test_phase(
                "Auth Tests",
                rpc_url,
                "tests/src/common/fixtures/auth-test.toml",
                "tests/src/common/fixtures/signers.toml",
                "8081",
                vec!["auth"],
                false,
            )
            .await
        }
    });

    join_set.spawn({
        let rpc_url = rpc_url.clone();
        async move {
            run_test_phase(
                "Payment Address Tests",
                rpc_url,
                "tests/src/common/fixtures/paymaster-address-test.toml",
                "tests/src/common/fixtures/signers.toml",
                "8082",
                vec!["payment_address"],
                true,
            )
            .await
        }
    });

    join_set.spawn({
        let rpc_url = rpc_url.clone();
        async move {
            run_test_phase(
                "Multi-Signer Tests",
                rpc_url,
                "tests/src/common/fixtures/kora-test.toml",
                "tests/src/common/fixtures/multi-signers.toml",
                "8083",
                vec!["multi_signer"],
                false,
            )
            .await
        }
    });

    let mut results = Vec::new();
    while let Some(result) = join_set.join_next().await {
        results.push(result?);
    }
    for result in results {
        result?;
    }

    Ok(())
}

/*
Output filtering with colors
*/
#[derive(Debug, Clone, Copy)]
enum OutputFilter {
    Test,
    CliCommand,
}

#[derive(Debug, Clone, Copy)]
pub enum TestPhaseColor {
    Regular,
    Auth,
    Payment,
    MultiSigner,
}

impl TestPhaseColor {
    fn from_port(port: &str) -> Self {
        match port {
            "8080" => Self::Regular,
            "8081" => Self::Auth,
            "8082" => Self::Payment,
            "8083" => Self::MultiSigner,
            _ => Self::Regular,
        }
    }

    fn ansi_code(&self) -> &'static str {
        match self {
            Self::Regular => "\x1b[32m",     // Green
            Self::Auth => "\x1b[34m",        // Blue
            Self::Payment => "\x1b[33m",     // Yellow
            Self::MultiSigner => "\x1b[35m", // Magenta
        }
    }

    fn reset_code() -> &'static str {
        "\x1b[0m"
    }

    fn colorize(&self, text: &str) -> String {
        format!("{}{}{}", self.ansi_code(), text, Self::reset_code())
    }
}

impl OutputFilter {
    fn should_show_line(&self, line: &str, show_verbose: bool) -> bool {
        match self {
            Self::Test => {
                line.starts_with("test ")
                    || line.contains("test result:")
                    || line.contains("running ")
                    || line.contains("FAILED")
                    || line.contains("failures:")
                    || line.contains("panicked")
                    || line.contains("assertion")
                    || line.contains("ERROR")
                    || line.trim().is_empty()
                    || (show_verbose
                        && (line.contains("Compiling")
                            || line.contains("Finished")
                            || line.contains("warning:")
                            || line.contains("error:")))
            }
            Self::CliCommand => {
                line.contains("ERROR")
                    || line.contains("error")
                    || line.contains("Failed")
                    || line.contains("Success")
                    || line.contains("✓")
                    || line.contains("✗")
                    || line.contains("Initialized")
                    || line.contains("Created")
                    || (show_verbose && line.contains("INFO"))
            }
        }
    }
}

fn filter_command_output(output: &str, filter: OutputFilter, show_verbose: bool) -> String {
    output
        .lines()
        .filter(|line| filter.should_show_line(line, show_verbose))
        .collect::<Vec<_>>()
        .join("\n")
}

fn filter_and_colorize_output(
    output: &str,
    filter: OutputFilter,
    show_verbose: bool,
    color: TestPhaseColor,
) -> String {
    let filtered = filter_command_output(output, filter, show_verbose);
    if filtered.is_empty() {
        filtered
    } else {
        color.colorize(&filtered)
    }
}

/*
Test execution
*/
pub async fn run_initialize_atas_for_kora_cli_tests(
    config_file: &str,
    rpc_url: &str,
    signers_config: &str,
    color: TestPhaseColor,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    println!("{}", color.colorize("• Initializing payment ATAs..."));

    let fee_payer_key = fs::read_to_string(AccountFile::FeePayer.local_key_path())?;

    let output = tokio::process::Command::new("cargo")
        .args([
            "run",
            "-p",
            "kora-cli",
            "--bin",
            "kora",
            "--",
            "--config",
            config_file,
            "--rpc-url",
            rpc_url,
            "rpc",
            "initialize-atas",
            "--signers-config",
            signers_config,
        ])
        .env("KORA_PRIVATE_KEY", fee_payer_key.trim())
        .output()
        .await?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let filtered_stderr = filter_command_output(&stderr, OutputFilter::CliCommand, false);
        if !filtered_stderr.is_empty() {
            println!("{filtered_stderr}");
        }
        return Err("Failed to initialize payment ATAs".into());
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let filtered_stdout = filter_command_output(&stdout, OutputFilter::CliCommand, false);
    if !filtered_stdout.is_empty() {
        println!("{filtered_stdout}");
    }
    println!("{}", color.colorize("• Payment ATAs ready"));

    Ok(())
}

pub async fn run_tests(
    port: &str,
    test_name: &str,
    color: TestPhaseColor,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let server_url = format!("http://127.0.0.1:{port}");

    let mut cmd = tokio::process::Command::new("cargo");

    cmd.args(["test", "-p", "tests", "--test", test_name, "--", "--nocapture"])
        .env(TEST_SERVER_URL_ENV, &server_url);

    for account_file in AccountFile::required_test_accounts_env_vars() {
        let (env_var, value) = account_file.get_as_env_var();
        cmd.env(env_var, value);
    }

    let output = cmd.output().await?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let filtered_stderr = filter_and_colorize_output(&stderr, OutputFilter::Test, false, color);
        if !filtered_stderr.is_empty() {
            println!("{filtered_stderr}");
        }
        return Err(format!("{test_name} tests failed").into());
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let filtered_stdout = filter_and_colorize_output(&stdout, OutputFilter::Test, false, color);
    println!("{filtered_stdout}");
    Ok(())
}

/*
Clean up
*/
pub async fn clean_up(
    test_runner: &mut TestRunner,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    println!("=== Cleaning up processes ===");

    if let Some(solana_test_validator_pid) = &mut test_runner.solana_test_validator_pid {
        solana_test_validator_pid.kill().await.ok();
        println!("Stopped Solana test validator");
    }

    for kora_pid in &mut test_runner.kora_pids {
        kora_pid.kill().await.ok();
    }

    println!("=== Cleanup complete ===");
    Ok(())
}

/*
Main function
*/

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mut test_runner = TestRunner {
        rpc_client: Arc::new(RpcClient::new(DEFAULT_RPC_URL.to_string())),
        reqwest_client: reqwest::Client::new(),
        solana_test_validator_pid: None,
        test_accounts: TestAccountInfo::default(),
        kora_pids: Vec::new(),
    };

    let result = async {
        setup_test_env(&mut test_runner).await?;
        run_all_test_phases(&test_runner).await?;
        Ok::<(), Box<dyn std::error::Error + Send + Sync>>(())
    }
    .await;

    clean_up(&mut test_runner).await?;
    result
}
