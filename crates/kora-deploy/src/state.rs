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
use serde::{Deserialize, Deserializer, Serialize};

// Keypairs are serialized as raw bytes in JSON; file security relies on OS-level permissions (see save_state_file()).
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
    #[serde(
        default,
        deserialize_with = "deserialize_wallet_pubkey",
        skip_serializing_if = "Option::is_none"
    )]
    pub wallet_pubkey: Option<Option<String>>,
}

fn deserialize_wallet_pubkey<'de, D>(
    deserializer: D,
) -> std::result::Result<Option<Option<String>>, D::Error>
where
    D: Deserializer<'de>,
{
    Deserialize::deserialize(deserializer).map(Some)
}

/// State files written before `chunk_size` was recorded used 900-byte chunks, and resuming
/// one at any other size would write at offsets that disagree with `written_chunks`.
fn legacy_chunk_size() -> usize {
    900
}

fn load_state_file<T: serde::de::DeserializeOwned>(
    path: &Path,
    type_name: &str,
) -> Result<Option<T>> {
    let data = match fs::read_to_string(path) {
        Ok(data) => data,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(err) => {
            return Err(err)
                .with_context(|| format!("failed to read {type_name} from {}", path.display()))
        }
    };
    let state = serde_json::from_str(&data)
        .with_context(|| format!("failed to parse {type_name} from {}", path.display()))?;
    Ok(Some(state))
}

fn save_state_file<T: Serialize>(state: &T, path: &Path, type_name: &str) -> Result<()> {
    let data = serde_json::to_string_pretty(state)
        .with_context(|| format!("failed to serialize {type_name}"))?;

    let temp_path = path.with_extension("tmp");

    let write_res = (|| -> Result<()> {
        let mut file = {
            #[cfg(unix)]
            {
                let mut options = fs::OpenOptions::new();
                options.write(true).create(true).truncate(true).mode(0o600);
                options.open(&temp_path)
            }
            #[cfg(not(unix))]
            {
                // share_mode(0) limits concurrent access, but does not enforce
                // file-level ACL restrictions (unlike Unix 0o600).
                fs::OpenOptions::new()
                    .write(true)
                    .create(true)
                    .truncate(true)
                    .share_mode(0)
                    .open(&temp_path)
            }
        }
        .with_context(|| {
            format!("failed to open temp {type_name} for writing: {}", temp_path.display())
        })?;

        file.write_all(data.as_bytes()).with_context(|| {
            format!("failed to write temp {type_name} to {}", temp_path.display())
        })?;
        file.sync_data().with_context(|| format!("failed to sync temp {type_name} to disk"))?;
        drop(file);

        fs::rename(&temp_path, path).with_context(|| {
            format!("failed to rename {} to {}", temp_path.display(), path.display())
        })?;

        Ok(())
    })();

    if write_res.is_err() {
        let _ = fs::remove_file(&temp_path);
    }

    write_res
}

impl DeployState {
    pub fn load(path: &Path) -> Result<Option<Self>> {
        load_state_file(path, "deploy state")
    }

    pub fn save(&self, path: &Path) -> Result<()> {
        save_state_file(self, path, "deploy state")
    }
}

// Buffer keypair is serialized as raw bytes in JSON; file security relies on OS-level permissions (see save_state_file()).
#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct UpgradeState {
    pub program: String,
    pub buffer_keypair: Vec<u8>,
    pub written_chunks: usize,
    pub kora_pubkey: String,
    pub program_hash: String,
    #[serde(default = "legacy_chunk_size")]
    pub chunk_size: usize,
    /// ProgramData slot before upgrade; verifies advancement on resume if buffer was closed after timeout.
    pub program_data_slot_at_start: u64,
}

impl UpgradeState {
    pub fn load(path: &Path) -> Result<Option<Self>> {
        load_state_file(path, "upgrade state")
    }

    pub fn save(&self, path: &Path) -> Result<()> {
        save_state_file(self, path, "upgrade state")
    }
}

impl ChunkWriterState for UpgradeState {
    fn written_chunks(&self) -> usize {
        self.written_chunks
    }

    fn set_written_chunks(&mut self, count: usize) {
        self.written_chunks = count;
    }

    fn program_hash(&self) -> &str {
        &self.program_hash
    }

    fn chunk_size(&self) -> usize {
        self.chunk_size
    }

    fn save(&self, path: &Path) -> Result<()> {
        self.save(path)
    }
}

/// Shared interface for chunk uploading and invariant checks across deploy and upgrade states.
pub trait ChunkWriterState: Send {
    fn written_chunks(&self) -> usize;
    fn set_written_chunks(&mut self, count: usize);
    fn program_hash(&self) -> &str;
    fn chunk_size(&self) -> usize;
    fn save(&self, path: &Path) -> Result<()>;
}

impl ChunkWriterState for DeployState {
    fn written_chunks(&self) -> usize {
        self.written_chunks
    }

    fn set_written_chunks(&mut self, count: usize) {
        self.written_chunks = count;
    }

    fn program_hash(&self) -> &str {
        &self.program_hash
    }

    fn chunk_size(&self) -> usize {
        self.chunk_size
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
    /// Preserves full path and extension to prevent collisions between distinct state files.
    pub fn lock_path(state_path: &Path) -> PathBuf {
        let mut os_str = state_path.as_os_str().to_os_string();
        os_str.push(".lock");
        PathBuf::from(os_str)
    }

    /// Fails fast if another process holds the state lock.
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
    #[cfg(unix)]
    use std::os::unix::fs::PermissionsExt;

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
            wallet_pubkey: Some(Some("TestWalletPubkey111111111111111111111111".to_string())),
        };

        let temp_file =
            temp_dir().join(format!("kora-deploy-state-test-{}.json", Pubkey::new_unique()));

        let _ = fs::remove_file(&temp_file);
        assert!(DeployState::load(&temp_file)?.is_none());

        state.save(&temp_file)?;

        let tmp_file = temp_file.with_extension("tmp");
        assert!(!tmp_file.exists(), "temporary file should be cleaned up after atomic rename");

        #[cfg(unix)]
        {
            let metadata = fs::metadata(&temp_file)?;
            assert_eq!(metadata.permissions().mode() & 0o777, 0o600);
        }

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
        assert_eq!(state.wallet_pubkey, None);
        Ok(())
    }

    #[test]
    fn test_deploy_state_wallet_pubkey_serde_backward_compat() -> Result<()> {
        let base_json = r#"{
            "program_keypair": [1],
            "buffer_keypair": [2],
            "program_data": "11111111111111111111111111111111",
            "written_chunks": 7,
            "kora_pubkey": "11111111111111111111111111111111",
            "program_hash": "dummyhash123""#;

        // Missing field -> None (old state file)
        let missing_state: DeployState = serde_json::from_str(&format!("{base_json}}}"))?;
        assert_eq!(missing_state.wallet_pubkey, None);
        let serialized_missing = serde_json::to_string(&missing_state)?;
        assert!(!serialized_missing.contains("wallet_pubkey"));

        // Explicit null -> Some(None) (immutable deployment)
        let null_state: DeployState =
            serde_json::from_str(&format!("{base_json}, \"wallet_pubkey\": null}}"))?;
        assert_eq!(null_state.wallet_pubkey, Some(None));
        let serialized_null = serde_json::to_string(&null_state)?;
        assert!(serialized_null.contains(r#""wallet_pubkey":null"#));

        // Explicit string -> Some(Some(pubkey)) (wallet-bound deployment)
        let wallet_state: DeployState =
            serde_json::from_str(&format!("{base_json}, \"wallet_pubkey\": \"wallet_abc\"}}"))?;
        assert_eq!(wallet_state.wallet_pubkey, Some(Some("wallet_abc".to_string())));
        let serialized_wallet = serde_json::to_string(&wallet_state)?;
        assert!(serialized_wallet.contains(r#""wallet_pubkey":"wallet_abc""#));

        Ok(())
    }

    #[test]
    fn test_upgrade_state_save_and_load() -> Result<()> {
        let state = UpgradeState {
            program: "Base58ProgramPubkey1111111111111111111111111".to_string(),
            buffer_keypair: vec![10, 20, 30, 40],
            written_chunks: 15,
            kora_pubkey: "Base58KoraPubkey111111111111111111111111111".to_string(),
            program_hash: "dummy_program_hash_987".to_string(),
            chunk_size: 3800,
            program_data_slot_at_start: 123_456_789,
        };

        let temp_file =
            temp_dir().join(format!("kora-upgrade-state-test-{}.json", Pubkey::new_unique()));

        let _ = fs::remove_file(&temp_file);
        assert!(UpgradeState::load(&temp_file)?.is_none());

        state.save(&temp_file)?;

        let tmp_file = temp_file.with_extension("tmp");
        assert!(!tmp_file.exists(), "temporary file should be cleaned up after atomic rename");

        #[cfg(unix)]
        {
            let metadata = fs::metadata(&temp_file)?;
            assert_eq!(metadata.permissions().mode() & 0o777, 0o600);
        }

        let loaded = UpgradeState::load(&temp_file)?.expect("upgrade state should exist");
        assert!(state == loaded);
        assert_eq!(loaded.program, state.program);
        assert_eq!(loaded.buffer_keypair, state.buffer_keypair);
        assert_eq!(loaded.written_chunks, state.written_chunks);
        assert_eq!(loaded.kora_pubkey, state.kora_pubkey);
        assert_eq!(loaded.program_hash, state.program_hash);
        assert_eq!(loaded.chunk_size, state.chunk_size);
        assert_eq!(loaded.program_data_slot_at_start, state.program_data_slot_at_start);

        // Overwrite state file (simulating progress updates in write_chunks).
        let mut state_updated = state.clone();
        state_updated.written_chunks = 30;
        state_updated.save(&temp_file)?;
        let reloaded =
            UpgradeState::load(&temp_file)?.expect("upgrade state should exist after overwrite");
        assert!(state_updated == reloaded);
        assert_eq!(reloaded.written_chunks, 30);

        fs::remove_file(&temp_file)?;

        Ok(())
    }

    #[test]
    fn test_upgrade_state_save_overwrites_existing_tmp_file() -> Result<()> {
        let state = UpgradeState {
            program: "Base58ProgramPubkey1111111111111111111111111".to_string(),
            buffer_keypair: vec![1, 2],
            written_chunks: 5,
            kora_pubkey: "Base58KoraPubkey111111111111111111111111111".to_string(),
            program_hash: "hash".to_string(),
            chunk_size: 900,
            program_data_slot_at_start: 100,
        };

        let temp_file = temp_dir()
            .join(format!("kora-upgrade-tmp-overwrite-test-{}.json", Pubkey::new_unique()));
        let tmp_file = temp_file.with_extension("tmp");

        // Simulate an orphaned .tmp file from a previous interrupted run.
        fs::write(&tmp_file, b"stale data")?;
        assert!(tmp_file.exists());

        state.save(&temp_file)?;
        assert!(!tmp_file.exists(), "temporary file should be renamed over destination");

        let loaded = UpgradeState::load(&temp_file)?.expect("state should exist");
        assert!(state == loaded);

        fs::remove_file(&temp_file)?;
        Ok(())
    }

    #[test]
    fn test_upgrade_state_chunk_writer_state_trait() -> Result<()> {
        let mut state = UpgradeState {
            program: "Base58ProgramPubkey1111111111111111111111111".to_string(),
            buffer_keypair: vec![1, 2],
            written_chunks: 10,
            kora_pubkey: "Base58KoraPubkey111111111111111111111111111".to_string(),
            program_hash: "hash".to_string(),
            chunk_size: 900,
            program_data_slot_at_start: 100,
        };

        fn exercise_chunk_writer<S: ChunkWriterState>(state: &mut S, path: &Path) -> Result<()> {
            assert_eq!(state.written_chunks(), 10);
            assert_eq!(state.program_hash(), "hash");
            assert_eq!(state.chunk_size(), 900);
            state.set_written_chunks(25);
            assert_eq!(state.written_chunks(), 25);
            state.save(path)
        }

        let temp_file =
            temp_dir().join(format!("kora-upgrade-trait-test-{}.json", Pubkey::new_unique()));
        exercise_chunk_writer(&mut state, &temp_file)?;

        let loaded = UpgradeState::load(&temp_file)?.expect("state should exist");
        assert_eq!(loaded.written_chunks, 25);

        fs::remove_file(&temp_file)?;
        Ok(())
    }

    #[test]
    fn test_upgrade_state_load_rejects_deploy_state_json() -> Result<()> {
        let deploy_json = r#"{
            "program_keypair": [1, 2],
            "buffer_keypair": [3, 4],
            "program_data": "dummy_program_data",
            "written_chunks": 5,
            "kora_pubkey": "dummy_kora_pubkey",
            "program_hash": "dummy_hash",
            "chunk_size": 900
        }"#;

        let temp_file =
            temp_dir().join(format!("kora-cross-state-test-{}.json", Pubkey::new_unique()));
        fs::write(&temp_file, deploy_json)?;

        let res = UpgradeState::load(&temp_file);
        assert!(
            res.is_err(),
            "UpgradeState must reject DeployState missing program and program_data_slot_at_start"
        );

        fs::remove_file(&temp_file)?;
        Ok(())
    }

    #[test]
    fn test_deploy_state_load_rejects_upgrade_state_json() -> Result<()> {
        let upgrade_json = r#"{
            "program": "Base58ProgramPubkey1111111111111111111111111",
            "buffer_keypair": [1, 2],
            "written_chunks": 5,
            "kora_pubkey": "Base58KoraPubkey111111111111111111111111111",
            "program_hash": "dummy_hash",
            "chunk_size": 900,
            "program_data_slot_at_start": 42
        }"#;

        let temp_file =
            temp_dir().join(format!("kora-cross-state-deploy-test-{}.json", Pubkey::new_unique()));
        fs::write(&temp_file, upgrade_json)?;

        let res = DeployState::load(&temp_file);
        assert!(
            res.is_err(),
            "DeployState must reject UpgradeState missing program_keypair and program_data"
        );

        fs::remove_file(&temp_file)?;
        Ok(())
    }

    #[test]
    fn test_upgrade_state_load_missing_file_returns_none() -> Result<()> {
        let missing_path =
            temp_dir().join(format!("kora-upgrade-missing-{}.json", Pubkey::new_unique()));
        assert!(UpgradeState::load(&missing_path)?.is_none());
        Ok(())
    }

    #[test]
    fn test_upgrade_state_load_malformed_json_returns_error() -> Result<()> {
        let malformed_path =
            temp_dir().join(format!("kora-upgrade-malformed-{}.json", Pubkey::new_unique()));
        fs::write(&malformed_path, b"invalid json content")?;
        let res = UpgradeState::load(&malformed_path);
        assert!(res.is_err());
        fs::remove_file(&malformed_path)?;
        Ok(())
    }

    #[test]
    fn test_upgrade_state_without_chunk_size_resumes_at_legacy_size() -> Result<()> {
        let state: UpgradeState = serde_json::from_str(
            r#"{
                "program": "11111111111111111111111111111111",
                "buffer_keypair": [2],
                "written_chunks": 7,
                "kora_pubkey": "11111111111111111111111111111111",
                "program_hash": "dummyhash123",
                "program_data_slot_at_start": 42
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
