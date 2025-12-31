use crate::{
    error::KoraError,
    signer::config::{SelectionStrategy, SignerConfig, SignerPoolConfig},
};
use rand::Rng;
use solana_keychain::{Signer, SolanaSigner};
use solana_sdk::pubkey::Pubkey;
use std::{
    collections::HashMap,
    str::FromStr,
    sync::{
        atomic::{AtomicU64, AtomicUsize, Ordering},
        Arc, RwLock,
    },
    time::{Duration, Instant},
};

const DEFAULT_WEIGHT: u32 = 1;

/// Circuit breaker health tracking per signer
#[derive(Debug, Clone, Default)]
struct SignerHealth {
    consecutive_failures: u32,
    last_failure_time: Option<Instant>,
    is_blacklisted: bool,
    blacklist_until: Option<Instant>,
}

/// Failover configuration for circuit breaker pattern
#[derive(Debug, Clone)]
pub struct FailoverConfig {
    pub failure_threshold: u32,
    pub blacklist_duration: Duration,
    pub max_retry_attempts: u32,
}

impl Default for FailoverConfig {
    fn default() -> Self {
        Self {
            failure_threshold: 3,
            blacklist_duration: Duration::from_secs(300),
            max_retry_attempts: 3,
        }
    }
}

/// Health metrics for monitoring signer status
#[derive(Debug, Clone)]
pub struct SignerHealthMetrics {
    pub name: String,
    pub public_key: String,
    pub is_healthy: bool,
    pub consecutive_failures: u32,
    pub last_failure_time: Option<Instant>,
}

/// Metadata associated with a signer in the pool
pub(crate) struct SignerWithMetadata {
    /// Human-readable name for this signer
    name: String,
    /// The actual signer instance
    signer: Arc<Signer>,
    /// Weight for weighted selection (higher = more likely to be selected)
    weight: u32,
    /// Timestamp of last use (Unix timestamp in seconds)
    last_used: AtomicU64,
}

impl Clone for SignerWithMetadata {
    fn clone(&self) -> Self {
        Self {
            name: self.name.clone(),
            signer: self.signer.clone(),
            weight: self.weight,
            last_used: AtomicU64::new(self.last_used.load(Ordering::Relaxed)),
        }
    }
}

impl SignerWithMetadata {
    /// Create a new signer with metadata
    pub(crate) fn new(name: String, signer: Arc<Signer>, weight: u32) -> Self {
        Self { name, signer, weight, last_used: AtomicU64::new(0) }
    }

    /// Update the last used timestamp to current time
    fn update_last_used(&self) {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        self.last_used.store(now, Ordering::Relaxed);
    }
}

/// Signer pool with automatic failover via circuit breaker pattern
pub struct SignerPool {
    signers: Vec<SignerWithMetadata>,
    strategy: SelectionStrategy,
    current_index: AtomicUsize,
    total_weight: u32,
    health_status: RwLock<HashMap<String, SignerHealth>>,
    failover_config: FailoverConfig,
}

/// Information about a signer for monitoring/debugging
#[derive(Debug, Clone)]
pub struct SignerInfo {
    pub public_key: String,
    pub name: String,
    pub weight: u32,
    pub last_used: u64, // Unix timestamp
}

impl SignerPool {
    #[cfg(test)]
    pub(crate) fn new(signers: Vec<SignerWithMetadata>) -> Self {
        let total_weight: u32 = signers.iter().map(|s| s.weight).sum();

        Self {
            signers,
            strategy: SelectionStrategy::RoundRobin,
            current_index: AtomicUsize::new(0),
            total_weight,
            health_status: RwLock::new(HashMap::new()),
            failover_config: FailoverConfig::default(),
        }
    }

    /// Create a new signer pool from configuration
    pub async fn from_config(config: SignerPoolConfig) -> Result<Self, KoraError> {
        if config.signers.is_empty() {
            return Err(KoraError::ValidationError("Cannot create empty signer pool".to_string()));
        }

        let mut signers = Vec::new();

        for signer_config in config.signers {
            log::info!("Initializing signer: {}", signer_config.name);

            let signer = SignerConfig::build_signer_from_config(&signer_config).await?;
            let weight = signer_config.weight.unwrap_or(DEFAULT_WEIGHT);

            signers.push(SignerWithMetadata::new(
                signer_config.name.clone(),
                Arc::new(signer),
                weight,
            ));

            log::info!(
                "Successfully initialized signer: {} (weight: {})",
                signer_config.name,
                weight
            );
        }

        let total_weight: u32 = signers.iter().map(|s| s.weight).sum();

        if matches!(config.signer_pool.strategy, SelectionStrategy::Weighted) && total_weight == 0 {
            return Err(KoraError::InternalServerError(
                "All signers have zero weight while using weighted selection strategy".to_string(),
            ));
        }

        log::info!(
            "Created signer pool with {} signers using {:?} strategy",
            signers.len(),
            config.signer_pool.strategy
        );

        Ok(Self {
            signers,
            strategy: config.signer_pool.strategy,
            current_index: AtomicUsize::new(0),
            total_weight,
            health_status: RwLock::new(HashMap::new()),
            failover_config: FailoverConfig::default(),
        })
    }

    /// Get the next signer according to the configured strategy
    ///
    /// Note: This method does NOT implement failover. Use `get_next_signer_with_failover()`
    /// for production code that needs automatic retry on signer failures.
    pub fn get_next_signer(&self) -> Result<Arc<Signer>, KoraError> {
        if self.signers.is_empty() {
            return Err(KoraError::InternalServerError("Signer pool is empty".to_string()));
        }

        let signer_meta = match self.strategy {
            SelectionStrategy::RoundRobin => self.round_robin_select(),
            SelectionStrategy::Random => self.random_select(),
            SelectionStrategy::Weighted => self.weighted_select(),
        }?;

        signer_meta.update_last_used();
        Ok(Arc::clone(&signer_meta.signer))
    }

    /// Get the next healthy signer with automatic failover
    ///
    /// This method implements a circuit breaker pattern:
    /// 1. Selects a signer using the configured strategy
    /// 2. Checks if the signer is healthy (not blacklisted)
    /// 3. If blacklisted, tries the next signer (up to max_retry_attempts)
    /// 4. Returns error if no healthy signers are available
    ///
    /// Use this method in production code to ensure high availability.
    pub fn get_next_signer_with_failover(&self) -> Result<(Arc<Signer>, String), KoraError> {
        if self.signers.is_empty() {
            return Err(KoraError::InternalServerError("Signer pool is empty".to_string()));
        }

        let max_attempts = self.failover_config.max_retry_attempts;
        let mut attempts = 0;

        while attempts < max_attempts {
            // Select candidate signer using configured strategy
            let signer_meta = match self.strategy {
                SelectionStrategy::RoundRobin => self.round_robin_select(),
                SelectionStrategy::Random => self.random_select(),
                SelectionStrategy::Weighted => self.weighted_select(),
            }?;

            match self.is_signer_healthy(&signer_meta.name) {
                Ok(true) => {
                    signer_meta.update_last_used();
                    return Ok((Arc::clone(&signer_meta.signer), signer_meta.name.clone()));
                }
                Ok(false) => {
                    log::warn!(
                        "Signer '{}' is blacklisted, selecting next candidate (attempt {}/{})",
                        signer_meta.name,
                        attempts + 1,
                        max_attempts
                    );
                    attempts += 1;
                }
                Err(e) => {
                    log::error!("Error checking signer '{}' health: {}", signer_meta.name, e);
                    attempts += 1;
                }
            }
        }

        Err(KoraError::InternalServerError(format!(
            "No healthy signers available after {} attempts",
            max_attempts
        )))
    }

    /// Check if signer is healthy and handle blacklist expiration
    fn is_signer_healthy(&self, signer_name: &str) -> Result<bool, KoraError> {
        let health_map = self.health_status.read().map_err(|_| {
            KoraError::InternalServerError("Failed to acquire health status read lock".to_string())
        })?;

        if let Some(health) = health_map.get(signer_name) {
            if health.is_blacklisted {
                if let Some(blacklist_until) = health.blacklist_until {
                    if Instant::now() > blacklist_until {
                        drop(health_map);
                        self.clear_blacklist(signer_name)?;
                        return Ok(true);
                    }
                }
                return Ok(false);
            }
        }

        Ok(true)
    }

    /// Record a signer failure and potentially blacklist it
    ///
    /// Call this method when a signer-specific error occurs (e.g., signing error,
    /// insufficient balance, RPC connection failure). The method tracks consecutive
    /// failures and blacklists the signer if it exceeds the threshold.
    ///
    /// # Arguments
    /// * `signer_name` - Name of the failing signer
    /// * `error` - The error that occurred (for logging)
    pub fn record_failure(&self, signer_name: &str, error: &KoraError) -> Result<(), KoraError> {
        let mut health_map = self.health_status.write().map_err(|_| {
            KoraError::InternalServerError("Failed to acquire health status write lock".to_string())
        })?;

        let health =
            health_map.entry(signer_name.to_string()).or_insert_with(SignerHealth::default);

        health.consecutive_failures += 1;
        health.last_failure_time = Some(Instant::now());

        log::error!("Signer '{}' failure #{}: {}", signer_name, health.consecutive_failures, error);

        // Blacklist if threshold exceeded
        if health.consecutive_failures >= self.failover_config.failure_threshold {
            health.is_blacklisted = true;
            health.blacklist_until = Some(Instant::now() + self.failover_config.blacklist_duration);

            log::warn!(
                "Signer '{}' blacklisted for {:?} after {} consecutive failures",
                signer_name,
                self.failover_config.blacklist_duration,
                health.consecutive_failures
            );
        }

        Ok(())
    }

    /// Record a successful operation and reset failure counters
    ///
    /// Call this method after a signer successfully completes an operation
    /// (e.g., signs a transaction). This resets the failure counter and
    /// indicates the signer has recovered.
    ///
    /// # Arguments
    /// * `signer_name` - Name of the successful signer
    pub fn record_success(&self, signer_name: &str) -> Result<(), KoraError> {
        let mut health_map = self.health_status.write().map_err(|_| {
            KoraError::InternalServerError("Failed to acquire health status write lock".to_string())
        })?;

        if let Some(health) = health_map.get_mut(signer_name) {
            if health.consecutive_failures > 0 {
                log::info!(
                    "Signer '{}' recovered after {} failures",
                    signer_name,
                    health.consecutive_failures
                );
            }
            health.consecutive_failures = 0;
            health.last_failure_time = None;
            health.is_blacklisted = false;
            health.blacklist_until = None;
        }

        Ok(())
    }

    /// Clear blacklist for a signer (used when blacklist expires)
    fn clear_blacklist(&self, signer_name: &str) -> Result<(), KoraError> {
        let mut health_map = self.health_status.write().map_err(|_| {
            KoraError::InternalServerError("Failed to acquire health status write lock".to_string())
        })?;

        if let Some(health) = health_map.get_mut(signer_name) {
            health.is_blacklisted = false;
            health.blacklist_until = None;
            log::info!("Signer '{}' blacklist cleared (expired)", signer_name);
        }

        Ok(())
    }

    /// Get health metrics for all signers (for monitoring/debugging)
    ///
    /// Returns a snapshot of health status for all signers in the pool.
    /// Use this for observability dashboards and operational monitoring.
    pub fn get_health_metrics(&self) -> Result<Vec<SignerHealthMetrics>, KoraError> {
        let health_map = self.health_status.read().map_err(|_| {
            KoraError::InternalServerError("Failed to acquire health status read lock".to_string())
        })?;

        let metrics: Vec<SignerHealthMetrics> = self
            .signers
            .iter()
            .map(|s| {
                let health = health_map.get(&s.name);
                SignerHealthMetrics {
                    name: s.name.clone(),
                    public_key: s.signer.pubkey().to_string(),
                    is_healthy: health.is_none_or(|h| !h.is_blacklisted),
                    consecutive_failures: health.map_or(0, |h| h.consecutive_failures),
                    last_failure_time: health.and_then(|h| h.last_failure_time),
                }
            })
            .collect();

        Ok(metrics)
    }

    /// Round-robin selection strategy
    fn round_robin_select(&self) -> Result<&SignerWithMetadata, KoraError> {
        let index = self.current_index.fetch_add(1, Ordering::AcqRel);
        let signer_index = index % self.signers.len();
        Ok(&self.signers[signer_index])
    }

    /// Random selection strategy
    fn random_select(&self) -> Result<&SignerWithMetadata, KoraError> {
        let mut rng = rand::rng();
        let index = rng.random_range(0..self.signers.len());
        Ok(&self.signers[index])
    }

    /// Weighted selection strategy (weighted random)
    fn weighted_select(&self) -> Result<&SignerWithMetadata, KoraError> {
        let mut rng = rand::rng();
        let mut target = rng.random_range(0..self.total_weight);

        for signer in &self.signers {
            if target < signer.weight {
                return Ok(signer);
            }
            target -= signer.weight;
        }

        // Fallback to first signer (shouldn't happen)
        Ok(&self.signers[0])
    }

    /// Get information about all signers in the pool
    pub fn get_signers_info(&self) -> Vec<SignerInfo> {
        self.signers
            .iter()
            .map(|s| SignerInfo {
                public_key: s.signer.pubkey().to_string(),
                name: s.name.clone(),
                weight: s.weight,
                last_used: s.last_used.load(Ordering::Relaxed),
            })
            .collect()
    }

    /// Get the number of signers in the pool
    pub fn len(&self) -> usize {
        self.signers.len()
    }

    /// Check if the pool is empty
    pub fn is_empty(&self) -> bool {
        self.signers.is_empty()
    }

    /// Get the configured strategy
    pub fn strategy(&self) -> &SelectionStrategy {
        &self.strategy
    }

    /// Get a signer by public key (for client consistency signer keys)
    pub fn get_signer_by_pubkey(&self, pubkey: &str) -> Result<Arc<Signer>, KoraError> {
        // Try to parse as Pubkey to validate format
        let target_pubkey = Pubkey::from_str(pubkey).map_err(|_| {
            KoraError::ValidationError(format!("Invalid signer signer key pubkey: {pubkey}"))
        })?;

        // Find signer with matching public key
        let signer_meta =
            self.signers.iter().find(|s| s.signer.pubkey() == target_pubkey).ok_or_else(|| {
                KoraError::ValidationError(format!("Signer with pubkey {pubkey} not found in pool"))
            })?;

        signer_meta.update_last_used();
        Ok(Arc::clone(&signer_meta.signer))
    }
}

#[cfg(test)]
mod tests {
    use solana_sdk::signature::Keypair;

    use super::*;
    use std::collections::HashMap;

    fn create_test_pool() -> SignerPool {
        // Create test signers using external signer library
        let keypair1 = Keypair::new();
        let keypair2 = Keypair::new();

        let external_signer1 =
            solana_keychain::Signer::from_memory(&keypair1.to_base58_string()).unwrap();
        let external_signer2 =
            solana_keychain::Signer::from_memory(&keypair2.to_base58_string()).unwrap();

        SignerPool {
            signers: vec![
                SignerWithMetadata::new("signer_1".to_string(), Arc::new(external_signer1), 1),
                SignerWithMetadata::new("signer_2".to_string(), Arc::new(external_signer2), 2),
            ],
            strategy: SelectionStrategy::RoundRobin,
            current_index: AtomicUsize::new(0),
            total_weight: 3,
            health_status: RwLock::new(HashMap::new()),
            failover_config: FailoverConfig::default(),
        }
    }

    #[test]
    fn test_round_robin_selection() {
        let pool = create_test_pool();

        // Test that round-robin cycles through signers
        let mut selections = HashMap::new();
        for _ in 0..100 {
            let signer = pool.get_next_signer().unwrap();
            *selections.entry(signer.pubkey().to_string()).or_insert(0) += 1;
        }

        // Should have selected both signers equally
        assert_eq!(selections.len(), 2);
        // Each signer should be selected 50 times
        assert!(selections.values().all(|&count| count == 50));
    }

    #[test]
    fn test_weighted_selection() {
        let mut pool = create_test_pool();
        pool.strategy = SelectionStrategy::Weighted;

        // Store the public keys for comparison (signer_1 has weight 1, signer_2 has weight 2)
        let signer1_pubkey = pool.signers[0].signer.pubkey().to_string();
        let signer2_pubkey = pool.signers[1].signer.pubkey().to_string();

        // Test weighted selection over many iterations
        let mut selections = HashMap::new();
        for _ in 0..300 {
            let signer = pool.get_next_signer().unwrap();
            *selections.entry(signer.pubkey().to_string()).or_insert(0) += 1;
        }

        // signer_2 has weight 2, signer_1 has weight 1
        // So signer_2 should be selected ~2/3 of the time
        let signer1_count = selections.get(&signer1_pubkey).unwrap_or(&0);
        let signer2_count = selections.get(&signer2_pubkey).unwrap_or(&0);

        // Allow some variance due to randomness
        assert!(*signer2_count > *signer1_count);
        assert!(*signer2_count > 150); // Should be around 200
        assert!(*signer1_count > 50); // Should be around 100
    }

    #[test]
    fn test_empty_pool() {
        let pool = SignerPool {
            signers: vec![],
            strategy: SelectionStrategy::RoundRobin,
            current_index: AtomicUsize::new(0),
            total_weight: 0,
            health_status: RwLock::new(HashMap::new()),
            failover_config: FailoverConfig::default(),
        };

        assert!(pool.get_next_signer().is_err());
        assert!(pool.is_empty());
        assert_eq!(pool.len(), 0);
    }

    #[test]
    fn test_failover_skips_blacklisted_signer() {
        let pool = create_test_pool();

        // Blacklist signer_1 by recording failures
        let error = KoraError::SigningError("Test error".to_string());
        for _ in 0..3 {
            pool.record_failure("signer_1", &error).unwrap();
        }

        // Verify signer_1 is blacklisted
        let metrics = pool.get_health_metrics().unwrap();
        let signer1_health = metrics.iter().find(|m| m.name == "signer_1").unwrap();
        assert!(!signer1_health.is_healthy);
        assert_eq!(signer1_health.consecutive_failures, 3);

        // Get next signer with failover - should skip signer_1
        let (signer, name) = pool.get_next_signer_with_failover().unwrap();
        assert_eq!(name, "signer_2");
        assert_eq!(signer.pubkey(), pool.signers[1].signer.pubkey());
    }

    #[test]
    fn test_record_success_resets_failures() {
        let pool = create_test_pool();

        // Record some failures
        let error = KoraError::SigningError("Test error".to_string());
        pool.record_failure("signer_1", &error).unwrap();
        pool.record_failure("signer_1", &error).unwrap();

        // Verify failures were recorded
        let metrics = pool.get_health_metrics().unwrap();
        let signer1_health = metrics.iter().find(|m| m.name == "signer_1").unwrap();
        assert_eq!(signer1_health.consecutive_failures, 2);

        // Record success
        pool.record_success("signer_1").unwrap();

        // Verify failures were reset
        let metrics = pool.get_health_metrics().unwrap();
        let signer1_health = metrics.iter().find(|m| m.name == "signer_1").unwrap();
        assert_eq!(signer1_health.consecutive_failures, 0);
        assert!(signer1_health.is_healthy);
    }

    #[test]
    fn test_blacklist_threshold() {
        let pool = create_test_pool();
        let error = KoraError::SigningError("Test error".to_string());

        // Record failures below threshold
        pool.record_failure("signer_1", &error).unwrap();
        pool.record_failure("signer_1", &error).unwrap();

        // Should not be blacklisted yet
        let metrics = pool.get_health_metrics().unwrap();
        let signer1_health = metrics.iter().find(|m| m.name == "signer_1").unwrap();
        assert!(signer1_health.is_healthy);
        assert_eq!(signer1_health.consecutive_failures, 2);

        // One more failure should trigger blacklist
        pool.record_failure("signer_1", &error).unwrap();

        // Should now be blacklisted
        let metrics = pool.get_health_metrics().unwrap();
        let signer1_health = metrics.iter().find(|m| m.name == "signer_1").unwrap();
        assert!(!signer1_health.is_healthy);
        assert_eq!(signer1_health.consecutive_failures, 3);
    }

    #[test]
    fn test_failover_with_all_signers_blacklisted() {
        let pool = create_test_pool();
        let error = KoraError::SigningError("Test error".to_string());

        // Blacklist all signers
        for _ in 0..3 {
            pool.record_failure("signer_1", &error).unwrap();
            pool.record_failure("signer_2", &error).unwrap();
        }

        // Attempt to get signer should fail
        let result = pool.get_next_signer_with_failover();
        assert!(result.is_err());
        if let Err(e) = result {
            assert!(e.to_string().contains("No healthy signers available"));
        }
    }

    #[test]
    fn test_get_health_metrics() {
        let pool = create_test_pool();
        let error = KoraError::SigningError("Test error".to_string());

        // Record some failures on signer_1
        pool.record_failure("signer_1", &error).unwrap();

        // Get metrics
        let metrics = pool.get_health_metrics().unwrap();
        assert_eq!(metrics.len(), 2);

        // Check signer_1 metrics
        let signer1_metrics = metrics.iter().find(|m| m.name == "signer_1").unwrap();
        assert_eq!(signer1_metrics.consecutive_failures, 1);
        assert!(signer1_metrics.is_healthy); // Not blacklisted yet
        assert!(signer1_metrics.last_failure_time.is_some());

        // Check signer_2 metrics (should be healthy with no failures)
        let signer2_metrics = metrics.iter().find(|m| m.name == "signer_2").unwrap();
        assert_eq!(signer2_metrics.consecutive_failures, 0);
        assert!(signer2_metrics.is_healthy);
        assert!(signer2_metrics.last_failure_time.is_none());
    }

    #[test]
    fn test_failover_config_custom_threshold() {
        let pool = SignerPool {
            signers: vec![SignerWithMetadata::new(
                "signer_1".to_string(),
                Arc::new(
                    solana_keychain::Signer::from_memory(&Keypair::new().to_base58_string())
                        .unwrap(),
                ),
                1,
            )],
            strategy: SelectionStrategy::RoundRobin,
            current_index: AtomicUsize::new(0),
            total_weight: 1,
            health_status: RwLock::new(HashMap::new()),
            failover_config: FailoverConfig {
                failure_threshold: 1, // Blacklist after just 1 failure
                blacklist_duration: Duration::from_secs(60),
                max_retry_attempts: 3,
            },
        };

        let error = KoraError::SigningError("Test error".to_string());

        // Single failure should blacklist with threshold=1
        pool.record_failure("signer_1", &error).unwrap();

        let metrics = pool.get_health_metrics().unwrap();
        let signer1_health = &metrics[0];
        assert!(!signer1_health.is_healthy);
        assert_eq!(signer1_health.consecutive_failures, 1);
    }
}
