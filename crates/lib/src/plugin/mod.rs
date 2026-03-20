mod gas_swap;

use crate::{
    config::{Config, PluginsConfig},
    error::KoraError,
    transaction::VersionedTransactionResolved,
};
use gas_swap::GasSwapPlugin;

pub trait TransactionPlugin: Send + Sync {
    fn name(&self) -> &str;
    fn validate(&self, transaction: &VersionedTransactionResolved) -> Result<(), KoraError>;
    fn validate_config(&self, _config: &Config) -> (Vec<String>, Vec<String>) {
        (vec![], vec![])
    }
}

pub struct PluginRegistry {
    plugins: Vec<Box<dyn TransactionPlugin>>,
}

impl PluginRegistry {
    pub fn from_config(config: &PluginsConfig) -> Result<Self, KoraError> {
        let mut plugins: Vec<Box<dyn TransactionPlugin>> = Vec::new();

        for name in &config.enabled {
            match name.as_str() {
                "GasSwap" => plugins.push(Box::new(GasSwapPlugin)),
                other => {
                    return Err(KoraError::InternalServerError(format!(
                        "Unknown plugin: '{other}'"
                    )))
                }
            }
        }

        Ok(Self { plugins })
    }

    pub fn run(&self, transaction: &VersionedTransactionResolved) -> Result<(), KoraError> {
        for plugin in &self.plugins {
            plugin.validate(transaction)?;
        }
        Ok(())
    }
}
