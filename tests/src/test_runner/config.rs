use serde::{Deserialize, Serialize};
use std::{collections::HashMap, fs, path::Path};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TestPhaseConfig {
    pub name: String,
    pub config: String,
    pub signers: String,
    pub port: String,
    pub tests: Vec<String>,
    #[serde(default)]
    pub initialize_payments_atas: bool,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct TestRunnerConfig {
    pub test: HashMap<String, TestPhaseConfig>,
}

impl TestRunnerConfig {
    pub fn load_from_file<P: AsRef<Path>>(
        path: P,
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let content = fs::read_to_string(path)?;
        let config: TestRunnerConfig = toml::from_str(&content)?;
        Ok(config)
    }

    pub fn get_all_phases(&self) -> Vec<(String, TestPhaseConfig)> {
        self.test.iter().map(|(key, config)| (key.clone(), config.clone())).collect()
    }

    pub fn get_phase(&self, phase_name: &str) -> Option<&TestPhaseConfig> {
        self.test.get(phase_name)
    }
}
