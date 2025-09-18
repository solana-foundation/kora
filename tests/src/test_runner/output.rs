#[derive(Debug)]
pub struct PhaseOutput {
    pub phase_name: String,
    pub output: String,
    pub success: bool,
}

#[derive(Debug, Clone, Copy)]
pub enum OutputFilter {
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
    pub fn from_port(port: &str) -> Self {
        match port {
            "8080" => Self::Regular,
            "8081" => Self::Auth,
            "8082" => Self::Payment,
            "8083" => Self::MultiSigner,
            _ => Self::Regular,
        }
    }

    pub fn ansi_code(&self) -> &'static str {
        match self {
            Self::Regular => "\x1b[32m",     // Green
            Self::Auth => "\x1b[34m",        // Blue
            Self::Payment => "\x1b[33m",     // Yellow
            Self::MultiSigner => "\x1b[35m", // Magenta
        }
    }

    pub fn reset_code() -> &'static str {
        "\x1b[0m"
    }

    pub fn colorize(&self, text: &str) -> String {
        format!("{}{}{}", self.ansi_code(), text, Self::reset_code())
    }
}

impl OutputFilter {
    pub fn should_show_line(&self, line: &str, show_verbose: bool) -> bool {
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

pub fn filter_command_output(output: &str, filter: OutputFilter, show_verbose: bool) -> String {
    output
        .lines()
        .filter(|line| filter.should_show_line(line, show_verbose))
        .collect::<Vec<_>>()
        .join("\n")
}

pub fn filter_and_colorize_output(
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
