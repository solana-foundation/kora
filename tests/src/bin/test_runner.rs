use clap::Parser;
use solana_client::nonblocking::rpc_client::RpcClient;
use std::{fs, sync::Arc};
use tests::{
    common::{
        constants::DEFAULT_RPC_URL, setup::TestAccountSetup, TestAccountInfo, TEST_SERVER_URL_ENV,
    },
    test_runner::{
        accounts::{
            download_accounts, set_environment_variables, set_lookup_table_environment_variables,
            AccountFile,
        },
        config::{TestPhaseConfig, TestRunnerConfig},
        kora::{is_kora_running, start_kora_rpc_server},
        output::{
            filter_and_colorize_output, filter_command_output, OutputFilter, PhaseOutput,
            TestPhaseColor,
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
}

impl TestRunner {
    pub fn new(rpc_url: String) -> Self {
        Self {
            rpc_client: Arc::new(RpcClient::new(rpc_url)),
            reqwest_client: reqwest::Client::new(),
            solana_test_validator_pid: None,
            test_accounts: TestAccountInfo::default(),
            kora_pids: Vec::new(),
        }
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
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let args = Args::parse();

    let mut test_runner = TestRunner::new(args.rpc_url.clone());
    let custom_rpc_url = args.rpc_url != DEFAULT_RPC_URL;

    let result = async {
        setup_test_env(&mut test_runner, args.force_refresh, custom_rpc_url).await?;
        run_all_test_phases(&test_runner, args.verbose, &args.config).await?;
        Ok::<(), Box<dyn std::error::Error + Send + Sync>>(())
    }
    .await;

    clean_up(&mut test_runner).await?;
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

    set_environment_variables()?;

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
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let rpc_url = test_runner.rpc_client.url();

    // Load test configuration
    let config = if std::path::Path::new(config_path).exists() {
        println!("Loading test configuration from: {config_path}");
        TestRunnerConfig::load_from_file(config_path)?
    } else {
        panic!("Test configuration file not found: {config_path}");
    };

    let mut join_set = JoinSet::new();

    // Spawn all test phases from config
    for (_, phase_config) in config.get_all_phases() {
        join_set.spawn({
            let rpc_url = rpc_url.clone();
            let phase_config = phase_config.clone();
            async move { run_test_phase_from_config(rpc_url, &phase_config, verbose).await }
        });
    }

    let mut phase_outputs = Vec::new();
    while let Some(result) = join_set.join_next().await {
        let phase_output = result?;
        phase_outputs.push(phase_output);
    }

    // Print all phase outputs in order
    phase_outputs.sort_by_key(|p| p.phase_name.clone());
    let mut all_success = true;
    for phase_output in phase_outputs {
        print!("{}", phase_output.output);
        if !phase_output.success {
            all_success = false;
        }
    }

    if !all_success {
        return Err("One or more test phases failed".into());
    }

    Ok(())
}

async fn run_test_phase_from_config(
    rpc_url: String,
    config: &TestPhaseConfig,
    verbose: bool,
) -> PhaseOutput {
    let test_names: Vec<&str> = config.tests.iter().map(|s| s.as_str()).collect();

    run_test_phase(
        &config.name,
        rpc_url,
        &config.config,
        &config.signers,
        &config.port,
        test_names,
        config.initialize_payments_atas,
        verbose,
    )
    .await
}

#[allow(clippy::too_many_arguments)]
pub async fn run_test_phase(
    phase_name: &str,
    rpc_url: String,
    config_file: &str,
    signers_config: &str,
    port: &str,
    test_names: Vec<&str>,
    initialize_payment_atas: bool,
    verbose: bool,
) -> PhaseOutput {
    let color = TestPhaseColor::from_port(port);
    let mut output = String::new();

    output.push_str(&color.colorize(&format!("=== Starting {phase_name} ===\n")));

    let mut kora_pid =
        match start_kora_rpc_server(rpc_url.clone(), config_file, signers_config, port).await {
            Ok(pid) => pid,
            Err(e) => {
                output.push_str(&color.colorize(&format!("Failed to start Kora server: {e}\n")));
                return PhaseOutput { phase_name: phase_name.to_string(), output, success: false };
            }
        };

    let mut attempts = 0;
    while !is_kora_running(port).await {
        attempts += 1;
        if attempts > 30 {
            output.push_str(&color.colorize(&format!(
                "Kora server failed to start on port {port} within 30 seconds\n"
            )));
            kora_pid.kill().await.ok();
            return PhaseOutput { phase_name: phase_name.to_string(), output, success: false };
        }
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
    }
    output.push_str(&color.colorize(&format!("Kora server started on port {port}\n")));

    let result = async {
        if initialize_payment_atas {
            run_initialize_atas_for_kora_cli_tests_buffered(
                config_file,
                &rpc_url,
                signers_config,
                color,
                &mut output,
            )
            .await?
        }

        for test_name in test_names {
            output
                .push_str(&color.colorize(&format!("Running {test_name} tests on port {port}\n")));
            run_tests_buffered(port, test_name, color, verbose, &mut output).await?
        }

        Ok::<(), Box<dyn std::error::Error + Send + Sync>>(())
    }
    .await;

    kora_pid.kill().await.ok();

    let success = result.is_ok();
    match &result {
        Ok(_) => output.push_str(&color.colorize(&format!("=== Completed {phase_name} ===\n"))),
        Err(e) => {
            output.push_str(&color.colorize(&format!("=== Failed {phase_name} - Error: {e} ===\n")))
        }
    }

    PhaseOutput { phase_name: phase_name.to_string(), output, success }
}

pub async fn run_initialize_atas_for_kora_cli_tests_buffered(
    config_file: &str,
    rpc_url: &str,
    signers_config: &str,
    color: TestPhaseColor,
    output: &mut String,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    output.push_str(&color.colorize("• Initializing payment ATAs...\n"));

    let fee_payer_key = fs::read_to_string(AccountFile::FeePayer.local_key_path())?;

    let cmd_output = tokio::process::Command::new("cargo")
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

    if !cmd_output.status.success() {
        let stderr = String::from_utf8_lossy(&cmd_output.stderr);
        let filtered_stderr = filter_command_output(&stderr, OutputFilter::CliCommand, false);
        if !filtered_stderr.is_empty() {
            output.push_str(&filtered_stderr);
            output.push('\n');
        }
        return Err("Failed to initialize payment ATAs".into());
    }

    let stdout = String::from_utf8_lossy(&cmd_output.stdout);
    let filtered_stdout = filter_command_output(&stdout, OutputFilter::CliCommand, false);
    if !filtered_stdout.is_empty() {
        output.push_str(&filtered_stdout);
        output.push('\n');
    }
    output.push_str(&color.colorize("• Payment ATAs ready\n"));

    Ok(())
}

pub async fn run_tests_buffered(
    port: &str,
    test_name: &str,
    color: TestPhaseColor,
    verbose: bool,
    output: &mut String,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let server_url = format!("http://127.0.0.1:{port}");

    let mut cmd = tokio::process::Command::new("cargo");

    cmd.args(["test", "-p", "tests", "--test", test_name, "--", "--nocapture"])
        .env(TEST_SERVER_URL_ENV, &server_url);

    for account_file in AccountFile::required_test_accounts_env_vars() {
        let (env_var, value) = account_file.get_as_env_var();
        cmd.env(env_var, value);
    }

    let cmd_output = cmd.output().await?;

    if !cmd_output.status.success() {
        let stderr = String::from_utf8_lossy(&cmd_output.stderr);
        let filtered_stderr =
            filter_and_colorize_output(&stderr, OutputFilter::Test, verbose, color);
        if !filtered_stderr.is_empty() {
            output.push_str(&filtered_stderr);
            output.push('\n');
        }
        return Err(format!("{test_name} tests failed").into());
    }

    let stdout = String::from_utf8_lossy(&cmd_output.stdout);
    let filtered_stdout = filter_and_colorize_output(&stdout, OutputFilter::Test, verbose, color);
    output.push_str(&filtered_stdout);
    output.push('\n');
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
