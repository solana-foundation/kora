use crate::{
    common::TEST_SERVER_URL_ENV,
    test_runner::{
        accounts::AccountFile,
        output::{filter_and_colorize_output, OutputFilter, TestPhaseColor},
    },
};

pub enum TestLanguage {
    Rust,
    TypeScript,
}

pub struct TestCommandHelper;

impl TestCommandHelper {
    pub async fn run_test(
        test_language: TestLanguage,
        test_name: &str,
        port: &str,
        color: TestPhaseColor,
        verbose: bool,
        output: &mut String,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        match test_language {
            TestLanguage::Rust => {
                Self::run_tests_buffered(test_name, port, color, verbose, output).await
            }
            TestLanguage::TypeScript => {
                Self::run_typescript_tests(test_name, port, color, verbose, output).await
            }
        }
    }

    async fn run_tests_buffered(
        test_name: &str,
        port: &str,
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
        let filtered_stdout =
            filter_and_colorize_output(&stdout, OutputFilter::Test, verbose, color);
        output.push_str(&filtered_stdout);
        output.push('\n');
        Ok(())
    }

    async fn run_typescript_tests(
        test_name: &str,
        port: &str,
        color: TestPhaseColor,
        verbose: bool,
        output: &mut String,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let pnpm_command = match test_name {
            "typescript_basic" => "test:integration",
            "typescript_auth" => "test:integration:auth",
            "typescript_turnkey" => "test:integration:turnkey",
            "typescript_privy" => "test:integration:privy",
            _ => return Err(format!("Unknown TypeScript test: {test_name}").into()),
        };

        let server_url = format!("http://127.0.0.1:{port}");

        let mut cmd = tokio::process::Command::new("pnpm");
        cmd.current_dir("sdks/ts").args(["run", pnpm_command]).env("KORA_RPC_URL", server_url);

        // Set auth flag based on test name
        if test_name == "typescript_auth" {
            cmd.env("ENABLE_AUTH", "true");
        }

        let cmd_output = cmd.output().await?;

        if !cmd_output.status.success() {
            let stderr = String::from_utf8_lossy(&cmd_output.stderr);
            let filtered_stderr =
                filter_and_colorize_output(&stderr, OutputFilter::TypeScript, verbose, color);
            if !filtered_stderr.is_empty() {
                output.push_str(&filtered_stderr);
                output.push('\n');
            }
            return Err(format!("{test_name} TypeScript tests failed").into());
        }

        let stdout = String::from_utf8_lossy(&cmd_output.stdout);
        let filtered_stdout =
            filter_and_colorize_output(&stdout, OutputFilter::TypeScript, verbose, color);
        output.push_str(&filtered_stdout);
        output.push('\n');
        Ok(())
    }
}
