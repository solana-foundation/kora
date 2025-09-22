use clap::Parser;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_commitment_config::CommitmentConfig;
use std::{collections::HashMap, sync::Arc, time::Instant};
use tests::{
    common::{constants::DEFAULT_RPC_URL, setup::TestAccountSetup, TestAccountInfo},
    test_runner::{
        accounts::{
            download_accounts, set_environment_variables, set_lookup_table_environment_variables,
            AccountFile,
        },
        commands::{TestCommandHelper, TestLanguage},
        config::{TestPhaseConfig, TestRunnerConfig},
        kora::{
            get_kora_binary_path, is_kora_running_with_client, release_port, start_kora_rpc_server,
        },
        output::{
            filter_command_output, limit_output_size, OutputFilter, PhaseOutput, TestPhaseColor,
        },
        validator::start_test_validator,
    },
};
use tokio::{process::Child, task::JoinSet};

pub struct TestRunner {
    pub rpc_client: Arc<RpcClient>,
    pub reqwest_client: reqwest::Client,
    pub solana_test_validator_pid: Option<Child>,
    pub test_accounts: TestAccountInfo,
    pub kora_pids: Vec<Child>,
    pub cached_keys: Arc<HashMap<AccountFile, String>>,
}

impl TestRunner {
    pub async fn new(rpc_url: String) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let mut cached_keys = HashMap::new();

        // Cache all required keys
        for &account_file in AccountFile::required_for_kora() {
            let key = tokio::fs::read_to_string(account_file.local_key_path()).await?;
            cached_keys.insert(account_file, key);
        }

        Ok(Self {
            rpc_client: Arc::new(RpcClient::new_with_commitment(
                rpc_url,
                CommitmentConfig::confirmed(),
            )),
            reqwest_client: reqwest::Client::new(),
            solana_test_validator_pid: None,
            test_accounts: TestAccountInfo::default(),
            kora_pids: Vec::new(),
            cached_keys: Arc::new(cached_keys),
        })
    }

    pub fn get_cached_key(
        &self,
        account_file: AccountFile,
    ) -> Result<&str, Box<dyn std::error::Error + Send + Sync>> {
        self.cached_keys
            .get(&account_file)
            .map(|s| s.as_str())
            .ok_or_else(|| format!("Key not found in cache: {account_file:?}").into())
    }
}

/*
CLI
*/
#[derive(Parser, Debug)]
#[command(name = "test_runner")]
#[command(about = "Kora integration test runner with configurable options")]
pub struct Args {
    /// Enable verbose output showing detailed test information
    #[arg(long, help = "Enable verbose output")]
    pub verbose: bool,

    /// RPC URL to use for Solana connection
    #[arg(
        long,
        default_value = DEFAULT_RPC_URL,
        help = "Solana RPC URL to connect to"
    )]
    pub rpc_url: String,

    /// Force refresh of test accounts, ignoring cached versions
    #[arg(long, help = "Skip loading cached accounts and setup test environment from scratch")]
    pub force_refresh: bool,

    /// Test configuration file
    #[arg(
        long,
        default_value = "tests/src/test_runner/test_cases.toml",
        help = "Path to test configuration file"
    )]
    pub config: String,

    /// Run only specific test phases (can be used multiple times)
    #[arg(
        long = "filter",
        help = "Run only specific test phases (e.g., --filter regular --filter auth)"
    )]
    pub filters: Vec<String>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let args = Args::parse();
    let start_time = Instant::now();

    println!("🚀 Starting test runner at {}", chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC"));

    let mut test_runner = TestRunner::new(args.rpc_url.clone()).await?;
    let custom_rpc_url = args.rpc_url != DEFAULT_RPC_URL;

    let (result, completed_phases) = async {
        setup_test_env(&mut test_runner, args.force_refresh, custom_rpc_url).await?;
        let phases =
            run_all_test_phases(&test_runner, args.verbose, &args.config, &args.filters).await?;
        Ok::<usize, Box<dyn std::error::Error + Send + Sync>>(phases)
    }
    .await
    .map_or_else(|e| (Err(e), 0), |phases| (Ok(()), phases));

    clean_up(&mut test_runner).await?;

    let total_duration = start_time.elapsed();
    println!(
        "✅ Test runner completed at {} ({} phases, Total time: {:.2}s)",
        chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC"),
        completed_phases,
        total_duration.as_secs_f64()
    );

    result
}

/*
Setting up test environment
*/

pub async fn setup_test_env_from_scratch(
) -> Result<TestAccountInfo, Box<dyn std::error::Error + Send + Sync>> {
    let mut setup = TestAccountSetup::new().await;
    let test_accounts = setup.setup_all_accounts().await?;

    Ok(test_accounts)
}

async fn setup_test_env(
    test_runner: &mut TestRunner,
    force_refresh: bool,
    custom_rpc_url: bool,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mut found_all_accounts = !force_refresh;

    if !force_refresh {
        for account_file in AccountFile::required_test_accounts() {
            if !account_file.test_account_path().exists() {
                found_all_accounts = false;
                break;
            }
        }
    }

    // Only start local validator if using default RPC URL
    if !custom_rpc_url {
        test_runner.solana_test_validator_pid =
            Some(start_test_validator(found_all_accounts).await?);
    } else {
        println!("Using external RPC, skipping local validator startup");
    }

    set_environment_variables(&test_runner.cached_keys)?;

    test_runner.test_accounts = setup_test_env_from_scratch().await?;

    if !found_all_accounts {
        download_accounts(&test_runner.rpc_client.clone(), &test_runner.test_accounts).await?;
    }
    set_lookup_table_environment_variables(&test_runner.test_accounts).await?;

    Ok(())
}

/*
Running Tests
*/

pub async fn run_all_test_phases(
    test_runner: &TestRunner,
    verbose: bool,
    config_path: &str,
    filters: &[String],
) -> Result<usize, Box<dyn std::error::Error + Send + Sync>> {
    let rpc_url = test_runner.rpc_client.url();

    // Load test configuration
    let config = if std::path::Path::new(config_path).exists() {
        println!("Loading test configuration from: {config_path}");
        TestRunnerConfig::load_from_file(config_path).await?
    } else {
        panic!("Test configuration file not found: {config_path}");
    };

    let mut join_set = JoinSet::new();

    // Spawn test phases from config (filtered if specified)
    for (phase_name, phase_config) in config.get_all_phases() {
        // Apply filter if specified
        if !filters.is_empty() && !filters.contains(&phase_name) {
            continue;
        }

        join_set.spawn({
            let rpc_url = rpc_url.clone();
            let phase_config = phase_config.clone();
            let cached_keys = test_runner.cached_keys.clone();
            let http_client = test_runner.reqwest_client.clone();
            async move {
                run_test_phase_from_config(
                    rpc_url,
                    &phase_config,
                    verbose,
                    cached_keys,
                    http_client,
                )
                .await
            }
        });
    }

    // Stream output as each test completes instead of waiting for all
    let mut all_success = true;
    let mut errors = Vec::new();
    let mut completed_phases = 0;

    while let Some(result) = join_set.join_next().await {
        match result {
            Ok(phase_output) => {
                completed_phases += 1;
                print!("{}", phase_output.output);

                if phase_output.truncated {
                    println!("⚠️  Output truncated for phase '{}'", phase_output.phase_name);
                }

                if !phase_output.success {
                    all_success = false;
                }
            }
            Err(e) => {
                println!("❌ Task failed: {e}");
                errors.push(e);
                all_success = false;
            }
        }
    }

    if !errors.is_empty() {
        return Err(format!("Multiple test phases failed: {errors:?}").into());
    }

    if !all_success {
        return Err("One or more test phases failed".into());
    }

    Ok(completed_phases)
}

async fn run_test_phase_from_config(
    rpc_url: String,
    config: &TestPhaseConfig,
    verbose: bool,
    cached_keys: Arc<HashMap<AccountFile, String>>,
    http_client: reqwest::Client,
) -> PhaseOutput {
    let test_names: Vec<&str> = config.tests.iter().map(|s| s.as_str()).collect();
    let preferred_port: u16 = config.port.parse().unwrap_or(8080);

    run_test_phase(
        &config.name,
        rpc_url,
        &config.config,
        &config.signers,
        test_names,
        config.initialize_payments_atas,
        verbose,
        cached_keys,
        http_client,
        preferred_port,
    )
    .await
}

#[allow(clippy::too_many_arguments)]
pub async fn run_test_phase(
    phase_name: &str,
    rpc_url: String,
    config_file: &str,
    signers_config: &str,
    test_names: Vec<&str>,
    initialize_payment_atas: bool,
    verbose: bool,
    cached_keys: Arc<HashMap<AccountFile, String>>,
    http_client: reqwest::Client,
    preferred_port: u16,
) -> PhaseOutput {
    let color = TestPhaseColor::from_phase_name(phase_name);
    let mut output = String::new();

    output
        .push_str(&color.colorize_with_controlled_flow(&format!("=== Starting {phase_name} ===")));

    let (mut kora_pid, actual_port) = match start_kora_rpc_server(
        rpc_url.clone(),
        config_file,
        signers_config,
        &cached_keys,
        preferred_port,
    )
    .await
    {
        Ok((pid, port)) => (pid, port),
        Err(e) => {
            output.push_str(
                &color.colorize_with_controlled_flow(&format!("Failed to start Kora server: {e}")),
            );
            let (limited_output, truncated) = limit_output_size(output);
            return PhaseOutput {
                phase_name: phase_name.to_string(),
                output: limited_output,
                success: false,
                truncated,
            };
        }
    };

    let mut attempts = 0;
    let mut delay = std::time::Duration::from_millis(50);
    let max_delay = std::time::Duration::from_secs(1);
    let max_attempts = 10;
    let port_str = actual_port.to_string();

    while !is_kora_running_with_client(&http_client, &port_str).await {
        attempts += 1;
        if attempts > max_attempts {
            output.push_str(&color.colorize_with_controlled_flow(&format!(
                "Kora server failed to start on port {actual_port} within {max_attempts} attempts"
            )));
            kora_pid.kill().await.ok();
            release_port(actual_port);
            let (limited_output, truncated) = limit_output_size(output);
            return PhaseOutput {
                phase_name: phase_name.to_string(),
                output: limited_output,
                success: false,
                truncated,
            };
        }

        tokio::time::sleep(delay).await;
        delay = std::cmp::min(delay * 2, max_delay);
    }
    output.push_str(
        &color.colorize_with_controlled_flow(&format!("Kora server started on port {actual_port}")),
    );

    let result = async {
        if initialize_payment_atas {
            run_initialize_atas_for_kora_cli_tests_buffered(
                config_file,
                &rpc_url,
                signers_config,
                color,
                &mut output,
                &cached_keys,
            )
            .await?
        }

        for test_name in test_names {
            output.push_str(&color.colorize_with_controlled_flow(&format!(
                "Running {test_name} tests on port {actual_port}"
            )));
            if test_name.starts_with("typescript_") {
                TestCommandHelper::run_test(
                    TestLanguage::TypeScript,
                    test_name,
                    &port_str,
                    color,
                    verbose,
                    &mut output,
                )
                .await?;
            } else {
                TestCommandHelper::run_test(
                    TestLanguage::Rust,
                    test_name,
                    &port_str,
                    color,
                    verbose,
                    &mut output,
                )
                .await?
            }
        }

        Ok::<(), Box<dyn std::error::Error + Send + Sync>>(())
    }
    .await;

    kora_pid.kill().await.ok();
    release_port(actual_port);

    let success = result.is_ok();
    match &result {
        Ok(_) => output.push_str(
            &color.colorize_with_controlled_flow(&format!("\n\n=== Completed {phase_name} ===")),
        ),
        Err(e) => output.push_str(&color.colorize_with_controlled_flow(&format!(
            "\n\n=== Failed {phase_name} - Error: {e} ==="
        ))),
    }

    let (limited_output, truncated) = limit_output_size(output);
    PhaseOutput { phase_name: phase_name.to_string(), output: limited_output, success, truncated }
}

pub async fn run_initialize_atas_for_kora_cli_tests_buffered(
    config_file: &str,
    rpc_url: &str,
    signers_config: &str,
    color: TestPhaseColor,
    output: &mut String,
    cached_keys: &Arc<HashMap<AccountFile, String>>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    output.push_str(&color.colorize_with_controlled_flow("• Initializing payment ATAs..."));

    let fee_payer_key =
        cached_keys.get(&AccountFile::FeePayer).ok_or("FeePayer key not found in cache")?;

    let kora_binary_path = get_kora_binary_path().await?;

    let cmd_output = tokio::process::Command::new(kora_binary_path)
        .args([
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

    if !cmd_output.status.success() {
        let stderr = String::from_utf8_lossy(&cmd_output.stderr);
        let filtered_stderr = filter_command_output(&stderr, OutputFilter::CliCommand, false);
        if !filtered_stderr.is_empty() {
            output.push_str(&filtered_stderr);
        }
        return Err("Failed to initialize payment ATAs".into());
    }

    let stdout = String::from_utf8_lossy(&cmd_output.stdout);
    let filtered_stdout = filter_command_output(&stdout, OutputFilter::CliCommand, false);
    if !filtered_stdout.is_empty() {
        output.push_str(&filtered_stdout);
    }
    output.push_str(&color.colorize_with_controlled_flow("• Payment ATAs ready"));

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
        if let Err(e) = solana_test_validator_pid.kill().await {
            println!("Failed to stop Solana test validator: {e}");
        } else {
            println!("Stopped Solana test validator");
        }
    }

    // Kill tracked Kora processes (though they're managed locally in each test phase)
    for kora_pid in &mut test_runner.kora_pids {
        if let Err(e) = kora_pid.kill().await {
            println!("Failed to stop Kora process: {e}");
        } else {
            println!("Stopped Kora process");
        }
    }

    println!("=== Cleanup complete ===");
    Ok(())
}
