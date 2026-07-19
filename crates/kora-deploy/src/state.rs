use std::{fs, path::Path};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DeployState {
    pub program_keypair: Vec<u8>,
    pub buffer_keypair: Vec<u8>,
    pub program_data: String,
    pub written_chunks: usize,
    pub kora_pubkey: String,
    pub program_hash: String,
}

impl DeployState {
    pub fn load(path: &Path) -> Result<Option<Self>> {
        if !path.exists() {
            return Ok(None);
        }
        let data = fs::read_to_string(path)
            .with_context(|| format!("failed to read deploy state from {}", path.display()))?;
        let state = serde_json::from_str(&data)
            .with_context(|| format!("failed to parse deploy state from {}", path.display()))?;
        Ok(Some(state))
    }

    pub fn save(&self, path: &Path) -> Result<()> {
        let data =
            serde_json::to_string_pretty(self).context("failed to serialize deploy state")?;

        let temp_path = path.with_extension("tmp");

        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            let mut options = fs::OpenOptions::new();
            options.write(true).create(true).truncate(true).mode(0o600);
            let mut file = options.open(&temp_path).with_context(|| {
                format!("failed to open temp deploy state for writing: {}", temp_path.display())
            })?;
            use std::io::Write;
            file.write_all(data.as_bytes()).with_context(|| {
                format!("failed to write temp deploy state to {}", temp_path.display())
            })?;
            file.sync_data().context("failed to sync temp deploy state to disk")?;
        }
        #[cfg(not(unix))]
        {
            let mut file = fs::File::create(&temp_path).with_context(|| {
                format!("failed to create temp deploy state: {}", temp_path.display())
            })?;
            use std::io::Write;
            file.write_all(data.as_bytes()).with_context(|| {
                format!("failed to write temp deploy state to {}", temp_path.display())
            })?;
            file.sync_data().context("failed to sync temp deploy state to disk")?;
        }

        fs::rename(&temp_path, path).with_context(|| {
            format!("failed to rename {} to {}", temp_path.display(), path.display())
        })?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{env::temp_dir, process};

    #[test]
    fn test_deploy_state_save_and_load() -> Result<()> {
        let state = DeployState {
            program_keypair: vec![1, 2, 3],
            buffer_keypair: vec![4, 5, 6],
            program_data: "Base58ProgramData11111111111111111111111111".to_string(),
            written_chunks: 42,
            kora_pubkey: "Base58KoraPubkey111111111111111111111111111".to_string(),
            program_hash: "dummyhash123".to_string(),
        };

        let temp_file = temp_dir().join(format!("kora-deploy-state-test-{}.json", process::id()));

        let _ = fs::remove_file(&temp_file);
        assert!(DeployState::load(&temp_file)?.is_none());

        state.save(&temp_file)?;

        let tmp_file = temp_file.with_extension("tmp");
        assert!(!tmp_file.exists(), "temporary file should be cleaned up after atomic rename");

        let loaded = DeployState::load(&temp_file)?.expect("state should exist");
        assert_eq!(state, loaded);

        fs::remove_file(&temp_file)?;

        Ok(())
    }
}
