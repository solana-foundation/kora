use std::fs;
use tempfile::NamedTempFile;

use crate::{config::Config, error::KoraError};

/// TOML-specific configuration builder for testing TOML parsing and serialization
///
/// This builder is specifically designed for testing TOML file generation and parsing.
/// For direct Config object creation in tests, use `config_mock::ConfigMockBuilder` instead.
///
/// Key differences:
/// - This builder generates TOML strings via `build_toml()`
/// - Used primarily for testing config file parsing logic
/// - Validates TOML parsing and deserialization
#[derive(Default)]
pub struct ConfigBuilder {
    validation: ValidationSection,
    kora: KoraSection,
    custom_sections: Vec<String>,
}

struct ValidationSection {
    max_allowed_lamports: u64,
    max_signatures: u64,
    allowed_programs: Vec<String>,
    allowed_tokens: Vec<String>,
    allowed_spl_paid_tokens: Vec<String>,
    disallowed_accounts: Vec<String>,
    price_source: String,
    price_config: Option<String>,
    token_2022_config: Option<String>,
}

struct KoraSection {
    rate_limit: u64,
    enabled_methods: Option<String>,
    cache_config: Option<String>,
}

impl Default for ValidationSection {
    fn default() -> Self {
        Self {
            max_allowed_lamports: 1000000000,
            max_signatures: 10,
            allowed_programs: vec!["program1".to_string()],
            allowed_tokens: vec!["token1".to_string()],
            allowed_spl_paid_tokens: vec!["token2".to_string()],
            disallowed_accounts: vec![],
            price_source: "Jupiter".to_string(),
            price_config: None,
            token_2022_config: None,
        }
    }
}

impl Default for KoraSection {
    fn default() -> Self {
        Self { rate_limit: 100, enabled_methods: None, cache_config: None }
    }
}

impl ConfigBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_programs(mut self, programs: Vec<&str>) -> Self {
        self.validation.allowed_programs = programs.iter().map(|s| s.to_string()).collect();
        self
    }

    pub fn with_tokens(mut self, tokens: Vec<&str>) -> Self {
        self.validation.allowed_tokens = tokens.iter().map(|s| s.to_string()).collect();
        self
    }

    pub fn with_spl_paid_tokens(mut self, tokens: Vec<&str>) -> Self {
        self.validation.allowed_spl_paid_tokens = tokens.iter().map(|s| s.to_string()).collect();
        self
    }

    pub fn with_disallowed_accounts(mut self, accounts: Vec<&str>) -> Self {
        self.validation.disallowed_accounts = accounts.iter().map(|s| s.to_string()).collect();
        self
    }

    pub fn with_margin_price(mut self, margin: f64) -> Self {
        self.validation.price_config =
            Some(format!("[validation.price]\ntype = \"margin\"\nmargin = {margin}\n"));
        self
    }

    pub fn with_fixed_price(mut self, amount: u64, token: &str) -> Self {
        self.validation.price_config = Some(format!(
            "[validation.price]\ntype = \"fixed\"\namount = {amount}\ntoken = \"{token}\"\n"
        ));
        self
    }

    pub fn with_free_price(mut self) -> Self {
        self.validation.price_config = Some("[validation.price]\ntype = \"free\"\n".to_string());
        self
    }

    pub fn with_invalid_price(mut self, price_type: &str) -> Self {
        self.validation.price_config =
            Some(format!("[validation.price]\ntype = \"{price_type}\"\nmargin = 0.1\n"));
        self
    }

    pub fn with_token2022_extensions(
        mut self,
        mint_exts: Vec<&str>,
        account_exts: Vec<&str>,
    ) -> Self {
        let mint_list =
            mint_exts.iter().map(|ext| format!("\"{ext}\"")).collect::<Vec<_>>().join(", ");
        let account_list =
            account_exts.iter().map(|ext| format!("\"{ext}\"")).collect::<Vec<_>>().join(", ");

        self.validation.token_2022_config = Some(format!(
            "[validation.token_2022]\nblocked_mint_extensions = [{mint_list}]\nblocked_account_extensions = [{account_list}]\n"
        ));
        self
    }

    pub fn with_enabled_methods(mut self, methods: &[(&str, bool)]) -> Self {
        let method_config = methods
            .iter()
            .map(|(name, enabled)| format!("{name} = {enabled}"))
            .collect::<Vec<_>>()
            .join("\n");

        self.kora.enabled_methods = Some(format!("[kora.enabled_methods]\n{method_config}\n"));
        self
    }

    pub fn with_cache_config(
        mut self,
        url: Option<&str>,
        enabled: bool,
        default_ttl: u64,
        account_ttl: u64,
    ) -> Self {
        let url_line = match url {
            Some(u) => format!("url = \"{u}\"\n"),
            None => String::new(),
        };

        self.kora.cache_config = Some(format!(
            "[kora.cache]\n{url_line}enabled = {enabled}\ndefault_ttl = {default_ttl}\naccount_ttl = {account_ttl}\n"
        ));
        self
    }

    pub fn build_toml(&self) -> String {
        let programs_list = self
            .validation
            .allowed_programs
            .iter()
            .map(|p| format!("\"{p}\""))
            .collect::<Vec<_>>()
            .join(", ");

        let tokens_list = self
            .validation
            .allowed_tokens
            .iter()
            .map(|t| format!("\"{t}\""))
            .collect::<Vec<_>>()
            .join(", ");

        let spl_tokens_list = self
            .validation
            .allowed_spl_paid_tokens
            .iter()
            .map(|t| format!("\"{t}\""))
            .collect::<Vec<_>>()
            .join(", ");

        let disallowed_list = if self.validation.disallowed_accounts.is_empty() {
            "[]".to_string()
        } else {
            format!(
                "[{}]",
                self.validation
                    .disallowed_accounts
                    .iter()
                    .map(|a| format!("\"{a}\""))
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        };

        let mut toml = format!(
            "[validation]\n\
            max_allowed_lamports = {}\n\
            max_signatures = {}\n\
            allowed_programs = [{}]\n\
            allowed_tokens = [{}]\n\
            allowed_spl_paid_tokens = [{}]\n\
            disallowed_accounts = {}\n\
            price_source = \"{}\"\n\n",
            self.validation.max_allowed_lamports,
            self.validation.max_signatures,
            programs_list,
            tokens_list,
            spl_tokens_list,
            disallowed_list,
            self.validation.price_source
        );

        if let Some(ref price_config) = self.validation.price_config {
            toml.push_str(&format!("{price_config}\n"));
        }

        if let Some(ref token_config) = self.validation.token_2022_config {
            toml.push_str(&format!("{token_config}\n"));
        }

        toml.push_str(&format!("[kora]\nrate_limit = {}\n", self.kora.rate_limit));

        if let Some(ref methods_config) = self.kora.enabled_methods {
            toml.push_str(&format!("{methods_config}\n"));
        }

        if let Some(ref cache_config) = self.kora.cache_config {
            toml.push_str(&format!("{cache_config}\n"));
        }

        for custom in &self.custom_sections {
            toml.push_str(&format!("{custom}\n"));
        }

        toml
    }

    pub fn build_config(&self) -> Result<Config, KoraError> {
        let toml_content = self.build_toml();
        let temp_file = NamedTempFile::new().unwrap();
        fs::write(&temp_file, toml_content).unwrap();
        Config::load_config(temp_file.path())
    }
}

/// Create an invalid config for testing error handling
/// Used specifically for testing TOML parsing error scenarios
pub fn create_invalid_config(content: &str) -> Result<Config, KoraError> {
    let temp_file = NamedTempFile::new().unwrap();
    fs::write(&temp_file, content).unwrap();
    Config::load_config(temp_file.path())
}
