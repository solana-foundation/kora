use serde::Deserialize;
use std::{fs, path::Path};
use toml;

use crate::common::{Feature, KoraError};

#[derive(Debug, Deserialize)]
pub struct Config {
    pub features: Features,
    pub tokens: Tokens,
}

#[derive(Debug, Deserialize)]
pub struct Features {
    pub enabled: Vec<Feature>,
}

#[derive(Debug, Deserialize)]
pub struct Tokens {
    pub allowed: Vec<String>,
}

pub fn load_config<P: AsRef<Path>>(path: P) -> Result<Config, KoraError> {
    let contents = fs::read_to_string(path).map_err(|e| {
        KoraError::InternalServerError(format!("Failed to read config file: {}", e))
    })?;

    toml::from_str(&contents)
        .map_err(|e| KoraError::InternalServerError(format!("Failed to parse config file: {}", e)))
}
