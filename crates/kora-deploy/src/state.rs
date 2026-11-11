#[cfg(unix)]
use std::os::unix::fs::OpenOptionsExt;
#[cfg(not(unix))]
use std::os::windows::fs::OpenOptionsExt;
use std::{
    fs::{self, TryLockError},
    io::Write,
    path::{Path, PathBuf},
};

use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};

// NOTE: keypairs are raw bytes in JSON; file security relies on OS-level permissions (see save()).
#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct DeployState {
    pub program_keypair: Vec<u8>,
    pub buffer_keypair: Vec<u8>,
    pub program_data: String,
    pub written_chunks: usize,
    pub kora_pubkey: String,
    pub program_hash: String,
    #[serde(default = "legacy_chunk_size")]
    pub chunk_size: usize,
}

/// State files written before `chunk_size` was recorded used 900-byte chunks, and resuming
/// one at any other size would write at offsets that disagree with `written_chunks`.
fn legacy_chunk_size() -> usize {
    900
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
            let mut options = fs::OpenOptions::new();
            options.write(true).create(true).truncate(true).mode(0o600);
            let mut file = options.open(&temp_path).with_context(|| {
                format!("failed to open temp deploy state for writing: {}", temp_path.display())
            })?;
            file.write_all(data.as_bytes()).with_context(|| {
                format!("failed to write temp deploy state to {}", temp_path.display())
            })?;
            file.sync_data().context("failed to sync temp deploy state to disk")?;
        }
        #[cfg(not(unix))]
        {
            // share_mode(0) limits concurrent access, but does not enforce
            // file-level ACL restrictions (unlike Unix 0o600).
            let mut file = fs::OpenOptions::new()
                .write(true)
                .create(true)
                .truncate(true)
                .share_mode(0)
                .open(&temp_path)
                .with_context(|| {
                    format!("failed to create temp deploy state: {}", temp_path.display())
                })?;
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

pub trait ChunkWriterState: Send {
    fn written_chunks(&self) -> usize;
    fn set_written_chunks(&mut self, count: usize);
    fn save(&self, path: &Path) -> Result<()>;
}

impl ChunkWriterState for DeployState {
    fn written_chunks(&self) -> usize {
        self.written_chunks
    }

    fn set_written_chunks(&mut self, count: usize) {
        self.written_chunks = count;
    }

    fn save(&self, path: &Path) -> Result<()> {
        self.save(path)
    }
}

/// Advisory file lock guard preventing multiple processes from operating on the same state file.
#[derive(Debug)]
#[must_use = "the lock is released when the guard is dropped"]
pub struct StateLockGuard {
    _file: fs::File,
}

impl StateLockGuard {
    /// Appends `.lock` to the full state file path to avoid colliding with `.tmp` files.
    pub fn lock_path(state_path: &Path) -> PathBuf {
        let mut os_str = state_path.as_os_str().to_os_string();
        os_str.push(".lock");
        PathBuf::from(os_str)
    }

    /// Acquires an advisory file lock on `<state_path>.lock`.
    pub fn acquire(state_path: &Path) -> Result<Self> {
        let lock_path = Self::lock_path(state_path);
        let file = fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(&lock_path)
            .with_context(|| {
                format!("failed to open state lock file at {}", lock_path.display())
            })?;

        if let Err(err) = file.try_lock() {
            return match err {
                TryLockError::WouldBlock => bail!(
                    "Another kora-deploy process is using state file {}. Failing fast.",
                    state_path.display()
                ),
                TryLockError::Error(io_err) => Err(anyhow::Error::new(io_err).context(format!(
                    "failed to acquire advisory lock on {}",
                    lock_path.display()
                ))),
            };
        }

        Ok(Self { _file: file })
    }
}

impl Drop for StateLockGuard {
    fn drop(&mut self) {
        if let Err(err) = self._file.unlock() {
            log::warn!("failed to release advisory lock on state file: {err}");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use solana_sdk::pubkey::Pubkey;
    use std::env::temp_dir;

    #[test]
    fn test_deploy_state_save_and_load() -> Result<()> {
        let state = DeployState {
            program_keypair: vec![1, 2, 3],
            buffer_keypair: vec![4, 5, 6],
            program_data: "Base58ProgramData11111111111111111111111111".to_string(),
            written_chunks: 42,
            kora_pubkey: "Base58KoraPubkey111111111111111111111111111".to_string(),
            program_hash: "dummyhash123".to_string(),
            chunk_size: 3800,
        };

        let temp_file =
            temp_dir().join(format!("kora-deploy-state-test-{}.json", Pubkey::new_unique()));

        let _ = fs::remove_file(&temp_file);
        assert!(DeployState::load(&temp_file)?.is_none());

        state.save(&temp_file)?;

        let tmp_file = temp_file.with_extension("tmp");
        assert!(!tmp_file.exists(), "temporary file should be cleaned up after atomic rename");

        let loaded = DeployState::load(&temp_file)?.expect("state should exist");
        assert!(state == loaded);

        fs::remove_file(&temp_file)?;

        Ok(())
    }

    #[test]
    fn test_deploy_state_without_chunk_size_resumes_at_legacy_size() -> Result<()> {
        let state: DeployState = serde_json::from_str(
            r#"{
                "program_keypair": [1],
                "buffer_keypair": [2],
                "program_data": "11111111111111111111111111111111",
                "written_chunks": 7,
                "kora_pubkey": "11111111111111111111111111111111",
                "program_hash": "dummyhash123"
            }"#,
        )?;
        assert_eq!(state.chunk_size, 900);
        Ok(())
    }

    #[test]
    fn test_state_lock_guard_contention_fails_fast_while_held() -> Result<()> {
        let state_path =
            temp_dir().join(format!("kora-deploy-state-lock-test-{}.json", Pubkey::new_unique()));
        let lock_path = StateLockGuard::lock_path(&state_path);

        let guard1 = StateLockGuard::acquire(&state_path)?;
        assert!(lock_path.exists());

        let err = StateLockGuard::acquire(&state_path).unwrap_err();
        let expected_msg = format!(
            "Another kora-deploy process is using state file {}. Failing fast.",
            state_path.display()
        );
        assert_eq!(err.to_string(), expected_msg);

        drop(guard1);
        let _ = fs::remove_file(&lock_path);
        Ok(())
    }

    #[test]
    fn test_state_lock_guard_released_on_drop_allows_reacquire() -> Result<()> {
        let state_path =
            temp_dir().join(format!("kora-deploy-state-lock-test-{}.json", Pubkey::new_unique()));
        let lock_path = StateLockGuard::lock_path(&state_path);

        let guard1 = StateLockGuard::acquire(&state_path)?;
        drop(guard1);

        let guard2 = StateLockGuard::acquire(&state_path)?;
        drop(guard2);

        let _ = fs::remove_file(&lock_path);
        Ok(())
    }

    #[test]
    fn test_state_lock_path_appends_dot_lock_and_differs_from_save_tmp() {
        let state_path = Path::new("/tmp/test-state.json");
        let lock_path = StateLockGuard::lock_path(state_path);
        let save_tmp_path = state_path.with_extension("tmp");

        assert_eq!(lock_path, Path::new("/tmp/test-state.json.lock"));
        assert_eq!(save_tmp_path, Path::new("/tmp/test-state.tmp"));
        assert_ne!(lock_path, save_tmp_path);
    }
}
