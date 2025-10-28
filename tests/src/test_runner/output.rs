pub const MAX_OUTPUT_SIZE: usize = 1024 * 1024; // 1MB limit

#[derive(Debug)]
pub struct PhaseOutput {
    pub phase_name: String,
    pub output: String,
    pub success: bool,
    pub truncated: bool,
}

#[derive(Debug, Clone, Copy)]
pub enum OutputFilter {
    Test,
    CliCommand,
    TypeScript,
}

#[derive(Debug, Clone, Copy)]
pub enum TestPhaseColor {
    Regular,
    Auth,
    Payment,
    MultiSigner,
    FeePayerPolicy,
    TypeScriptBasic,
    TypeScriptAuth,
    TypeScriptTurnkey,
    TypeScriptPrivy,
}

impl TestPhaseColor {
    pub fn from_phase_name(phase_name: &str) -> Self {
        match phase_name {
            "Regular Integration Tests" => Self::Regular,
            "Auth Tests" => Self::Auth,
            "Payment Address Tests" => Self::Payment,
            "Free Signing Tests" => Self::Regular,
            "Multi-Signer Tests" => Self::MultiSigner,
            "Fee Payer Policy Tests" => Self::FeePayerPolicy,
            name if name.starts_with("TypeScript") => Self::from_typescript_phase(name),
            name if name.starts_with("typescript_") => Self::from_typescript_phase(name),
            // Fallback patterns
            name if name.to_lowercase().contains("auth") => Self::Auth,
            name if name.to_lowercase().contains("payment") => Self::Payment,
            name if name.to_lowercase().contains("multi") => Self::MultiSigner,
            name if name.to_lowercase().contains("typescript") => Self::TypeScriptBasic,
            _ => Self::Regular,
        }
    }

    fn from_typescript_phase(name: &str) -> Self {
        match name {
            "typescript_basic" => Self::TypeScriptBasic,
            "typescript_auth" => Self::TypeScriptAuth,
            "typescript_turnkey" => Self::TypeScriptTurnkey,
            "typescript_privy" => Self::TypeScriptPrivy,
            _ => Self::TypeScriptBasic,
        }
    }

    pub fn ansi_code(&self) -> &'static str {
        match self {
            Self::Regular => "\x1b[32m",     // Green
            Self::Auth => "\x1b[34m",        // Blue
            Self::Payment => "\x1b[33m",     // Yellow
            Self::MultiSigner => "\x1b[35m", // Magenta
            Self::FeePayerPolicy => "\x1b[39m",
            Self::TypeScriptBasic => "\x1b[36m",   // Cyan
            Self::TypeScriptAuth => "\x1b[31m",    // Red
            Self::TypeScriptTurnkey => "\x1b[37m", // White
            Self::TypeScriptPrivy => "\x1b[90m",   // Gray
        }
    }

    pub fn reset_code() -> &'static str {
        "\x1b[0m"
    }

    pub fn colorize(&self, text: &str) -> String {
        format!("{}{}{}", self.ansi_code(), text, Self::reset_code())
    }

    pub fn colorize_with_controlled_flow(&self, text: &str) -> String {
        // Remove all existing newlines and add controlled ones with proper spacing
        let cleaned_text = text.replace('\n', "");
        let controlled_text = format!("{cleaned_text}\n\n");
        format!("{}{}{}", self.ansi_code(), controlled_text, Self::reset_code())
    }
}

impl OutputFilter {
    pub fn should_show_line(&self, line: &str, show_verbose: bool) -> bool {
        match self {
            Self::Test => {
                //
                line.contains("test result:")
                    || line.contains("FAILED")
                    || line.contains("failures:")
                    || line.contains("panicked")
                    || line.contains("assertion")
                    || line.contains("ERROR")
                    || (show_verbose
                        && (line.contains("Compiling")
                            || line.contains("running ")
                            || line.starts_with("test ")
                            || line.contains("Finished")
                            || line.contains("warning:")
                            || line.contains("error:")))
            }
            Self::CliCommand => {
                line.contains("ERROR")
                    || line.contains("error")
                    || line.contains("Failed")
                    || line.contains("Success")
                    || line.contains("✗")
                    || (show_verbose
                        && (line.contains("INFO")
                            || line.contains("✓")
                            || line.contains("Initialized")
                            || line.contains("Created")))
            }
            Self::TypeScript => {
                // Jest and TypeScript test output patterns
                line.contains("PASS")
                    || line.contains("FAIL")
                    || line.contains("✓")
                    || line.contains("✗")
                    || line.contains("Tests:")
                    || line.contains("Test Suites:")
                    || line.contains("Snapshots:")
                    || line.contains("Time:")
                    || line.contains("Ran all test suites")
                    || line.contains("Test results:")
                    || line.contains("expect")
                    || line.contains("Error:")
                    || line.contains("AssertionError")
                    || line.contains("TypeError")
                    || line.contains("ReferenceError")
                    || line.contains("failed with exit code")
                    || line.contains("npm ERR!")
                    || line.contains("pnpm ERR!")
                    || (show_verbose
                        && (line.contains("Running")
                            || line.contains("Starting")
                            || line.contains("Finished")))
            }
        }
    }
}

pub fn filter_command_output(output: &str, filter: OutputFilter, show_verbose: bool) -> String {
    // If verbose, show everything without filtering
    if show_verbose {
        return clean_multiple_newlines(output);
    }

    // Otherwise apply pattern filtering
    let filtered = output
        .lines()
        .filter(|line| filter.should_show_line(line, show_verbose))
        .collect::<Vec<_>>()
        .join("\n");

    clean_multiple_newlines(&filtered)
}

fn clean_multiple_newlines(text: &str) -> String {
    // Replace multiple consecutive newlines with single newlines
    let mut result = text.to_string();
    while result.contains("\n\n\n") {
        result = result.replace("\n\n\n", "\n\n");
    }
    result.trim_end().to_string()
}

pub fn filter_and_colorize_output(
    output: &str,
    filter: OutputFilter,
    show_verbose: bool,
    color: TestPhaseColor,
) -> String {
    let filtered = filter_command_output(output, filter, show_verbose);
    color.colorize(&filtered)
}

pub fn limit_output_size(output: String) -> (String, bool) {
    if output.len() > MAX_OUTPUT_SIZE {
        let truncated_output = format!(
            "{}... (truncated {} bytes)",
            &output[..MAX_OUTPUT_SIZE],
            output.len() - MAX_OUTPUT_SIZE
        );
        (truncated_output, true)
    } else {
        (output, false)
    }
}
