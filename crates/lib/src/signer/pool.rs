use crate::{
    error::KoraError,
    signer::config::{SelectionStrategy, SignerConfig, SignerPoolConfig},
};
use parking_lot::RwLock;
use rand::Rng;
use solana_keychain::{Signer, SolanaSigner};
use solana_sdk::pubkey::Pubkey;
use std::{
    collections::HashMap,
    str::FromStr,
    sync::{
        atomic::{AtomicU64, AtomicUsize, Ordering},
        Arc,
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
    failover_enabled: bool,
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
            failover_enabled: false,
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
            failover_enabled: config.signer_pool.failover_enabled,
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

        // Simple non-failover get_next_signer also needs to function.
        // We can pass an empty map here too so it acts like "blind" selection.
        let empty_map = HashMap::new();
        let signer_meta = match self.strategy {
            SelectionStrategy::RoundRobin => self.round_robin_select(&empty_map),
            SelectionStrategy::Random => self.random_select(&empty_map),
            SelectionStrategy::Weighted => self.weighted_select(&empty_map),
        }?;

        signer_meta.update_last_used();
        Ok(Arc::clone(&signer_meta.signer))
    }

    /// Get the next healthy signer with automatic failover
    ///
    /// This method implements a circuit breaker pattern:
    /// 1. Selects a signer using the configured strategy
    /// 2. If failover is DISABLED, returns the signer immediately (even if unhealthy)
    /// 3. If failover is ENABLED, checks if the signer is healthy (not blacklisted)
    /// 4. If blacklisted, tries the next signer (up to max_retry_attempts)
    /// 5. Returns error if no healthy signers are available
    ///
    /// Use this method in production code to ensure high availability.
    pub fn get_next_signer_with_failover(&self) -> Result<(Arc<Signer>, String), KoraError> {
        if self.signers.is_empty() {
            return Err(KoraError::InternalServerError("Signer pool is empty".to_string()));
        }

        // If failover is disabled, just pick one and return it
        if !self.failover_enabled {
            // Read lock needed for filters even if failover disabled, or we pass empty map/always true?
            // Existing logic didn't filter. To keep "failover disabled" behavior pure, we should ignore health.
            // But we changed the signatures of select methods.
            // We can just pass the health map, but the selection logic now filters.
            // Wait, if failover is disabled, we probably want the original behavior (just pick one, even if blacklisted).
            // Reverting selection methods to accept optional map or making separate internal methods is cleaner.
            // OR we just pass the map but ignore blacklist in the check if failover is disabled?
            // Actually, the requirements say "Update selection strategies to EXCLUDE blacklisted...".
            // It implies we should always exclude them in the internal selection logic used by the loop.
            // But for this "failover disabled" path, we just want "a signer".

            // Let's create a dummy empty map so everything looks healthy for this call?
            let empty_map = HashMap::new();
            // Using empty map means is_signer_healthy_internal returns true (not found in map = healthy).

            let signer_meta = match self.strategy {
                SelectionStrategy::RoundRobin => self.round_robin_select(&empty_map),
                SelectionStrategy::Random => self.random_select(&empty_map),
                SelectionStrategy::Weighted => self.weighted_select(&empty_map),
            }?;
            signer_meta.update_last_used();
            return Ok((Arc::clone(&signer_meta.signer), signer_meta.name.clone()));
        }
        let max_attempts = self.failover_config.max_retry_attempts;
        let mut attempts = 0;

        // Get a read lock on health status to check blacklist efficiently during selection
        let health_map = self.health_status.read();

        while attempts < max_attempts {
            // Select candidate signer using configured strategy, filtering out blacklisted ones
            let signer_meta = match self.strategy {
                SelectionStrategy::RoundRobin => self.round_robin_select(&health_map),
                SelectionStrategy::Random => self.random_select(&health_map),
                SelectionStrategy::Weighted => self.weighted_select(&health_map),
            }?;

            // If we selected a signer, double check strictly (though strategy should have filtered)
            // and return it. The strategy guarantees we try to pick a non-blacklisted one.
            if self.is_signer_healthy_internal(&signer_meta.name, &health_map) {
                signer_meta.update_last_used();
                return Ok((Arc::clone(&signer_meta.signer), signer_meta.name.clone()));
            }

            // If we somehow got a blacklisted signer (or health check failed), log and retry.
            // This might happen if 'is_signer_healthy_internal' has extra logic like expiration check
            // that the simple filter didn't catch, or if all signers are blacklisted.
            log::warn!(
                "Selected signer '{}' was unhealthy, selecting next candidate (attempt {}/{})",
                signer_meta.name,
                attempts + 1,
                max_attempts
            );
            attempts += 1;
        }

        Err(KoraError::InternalServerError(format!(
            "No healthy signers available after {} attempts",
            max_attempts
        )))
    }

    /// Check if signer is healthy and handle blacklist expiration
    /// uses internal helper to avoid lock recursion
    #[allow(dead_code)]
    fn is_signer_healthy(&self, signer_name: &str) -> Result<bool, KoraError> {
        let health_map = self.health_status.upgradable_read();

        if let Some(health) = health_map.get(signer_name) {
            if health.is_blacklisted {
                if let Some(blacklist_until) = health.blacklist_until {
                    if Instant::now() > blacklist_until {
                        // Upgrade to write lock to clear blacklist
                        let mut health_map_write =
                            parking_lot::RwLockUpgradableReadGuard::upgrade(health_map);
                        if let Some(health_mut) = health_map_write.get_mut(signer_name) {
                            health_mut.is_blacklisted = false;
                            health_mut.blacklist_until = None;
                            log::info!("Signer '{}' blacklist cleared (expired)", signer_name);
                        }
                        return Ok(true);
                    }
                }
                return Ok(false);
            }
        }
        Ok(true)
    }

    /// Internal health check that takes a reference to the map (no locking)
    /// Returns true if healthy (not blacklisted), false otherwise.
    /// Does NOT handle expiration - that requires write access.
    fn is_signer_healthy_internal(
        &self,
        signer_name: &str,
        health_map: &HashMap<String, SignerHealth>,
    ) -> bool {
        if let Some(health) = health_map.get(signer_name) {
            if health.is_blacklisted {
                // Check expiration "read-only" style - if expired, we treat as healthy CANDIDATE
                // efficiently, but actual clearing happens elsewhere or lazily.
                // For selection logic, if it's expired, we can try to use it.
                if let Some(blacklist_until) = health.blacklist_until {
                    if Instant::now() > blacklist_until {
                        return true;
                    }
                }
                return false;
            }
        }
        true
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
        let health_map = self.health_status.upgradable_read();

        // Check if we need to insert a new entry, if so upgrade immediately
        if !health_map.contains_key(signer_name) {
            let mut health_map_write = parking_lot::RwLockUpgradableReadGuard::upgrade(health_map);
            health_map_write.insert(signer_name.to_string(), SignerHealth::default());
            // Downgrade not supported directly in standard parking_lot without scope management,
            // but we can just use the write lock for the rest of this function call or re-acquire.
            // For simplicity, we'll continue with the write lock logic.
            let health = health_map_write.get_mut(signer_name).unwrap();
            Self::update_health_entry(health, signer_name, error, &self.failover_config);
            return Ok(());
        }

        // Pass upgradable guard to logic that might upgrade
        let mut health_map_write = parking_lot::RwLockUpgradableReadGuard::upgrade(health_map);
        if let Some(health) = health_map_write.get_mut(signer_name) {
            Self::update_health_entry(health, signer_name, error, &self.failover_config);
        }

        Ok(())
    }

    fn update_health_entry(
        health: &mut SignerHealth,
        signer_name: &str,
        error: &KoraError,
        config: &FailoverConfig,
    ) {
        health.consecutive_failures += 1;
        health.last_failure_time = Some(Instant::now());

        log::error!("Signer '{}' failure #{}: {}", signer_name, health.consecutive_failures, error);

        // Blacklist if threshold exceeded
        if health.consecutive_failures >= config.failure_threshold {
            health.is_blacklisted = true;
            health.blacklist_until = Some(Instant::now() + config.blacklist_duration);

            log::warn!(
                "Signer '{}' blacklisted for {:?} after {} consecutive failures",
                signer_name,
                config.blacklist_duration,
                health.consecutive_failures
            );
        }
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
        let health_map = self.health_status.upgradable_read();

        if let Some(health) = health_map.get(signer_name) {
            // Only upgrade if we actually need to change state (has failures to clear)
            if health.consecutive_failures > 0 || health.is_blacklisted {
                let mut health_map_write =
                    parking_lot::RwLockUpgradableReadGuard::upgrade(health_map);
                if let Some(health_mut) = health_map_write.get_mut(signer_name) {
                    if health_mut.consecutive_failures > 0 {
                        log::info!(
                            "Signer '{}' recovered after {} failures",
                            signer_name,
                            health_mut.consecutive_failures
                        );
                    }
                    health_mut.consecutive_failures = 0;
                    health_mut.last_failure_time = None;
                    health_mut.is_blacklisted = false;
                    health_mut.blacklist_until = None;
                }
            }
        }

        Ok(())
    }

    /// Get health metrics for all signers (for monitoring/debugging)
    ///
    /// Returns a snapshot of health status for all signers in the pool.
    /// Use this for observability dashboards and operational monitoring.
    pub fn get_health_metrics(&self) -> Result<Vec<SignerHealthMetrics>, KoraError> {
        let health_map = self.health_status.read();

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

    /// Round-robin selection strategy with blacklist filtering
    fn round_robin_select(
        &self,
        health_map: &HashMap<String, SignerHealth>,
    ) -> Result<&SignerWithMetadata, KoraError> {
        // We try to find a healthy signer starting from the current index
        // We limit iterations to scan the whole list once to avoid infinite loops if all are blacklisted
        let start_index = self.current_index.fetch_add(1, Ordering::AcqRel);

        for i in 0..self.signers.len() {
            let idx = (start_index + i) % self.signers.len();
            let signer = &self.signers[idx];

            if self.is_signer_healthy_internal(&signer.name, health_map) {
                return Ok(signer);
            }
        }

        // If all are blacklisted, fallback to standard round robin (let the caller handle the failure/retry logic or just fail)
        // But the requirement implies we should try to find a healthy one.
        // If we found none above, we return the one at the start index just to return something,
        // and the health check in the caller will fail/log it.
        let idx = start_index % self.signers.len();
        Ok(&self.signers[idx])
    }

    /// Random selection strategy with blacklist filtering
    fn random_select(
        &self,
        health_map: &HashMap<String, SignerHealth>,
    ) -> Result<&SignerWithMetadata, KoraError> {
        let mut rng = rand::rng();

        // Create a list of indices of healthy signers
        let healthy_indices: Vec<usize> = self
            .signers
            .iter()
            .enumerate()
            .filter(|(_, s)| self.is_signer_healthy_internal(&s.name, health_map))
            .map(|(i, _)| i)
            .collect();

        if !healthy_indices.is_empty() {
            let rand_idx = rng.random_range(0..healthy_indices.len());
            return Ok(&self.signers[healthy_indices[rand_idx]]);
        }

        // Fallback to random if all are unhealthy
        let index = rng.random_range(0..self.signers.len());
        Ok(&self.signers[index])
    }

    /// Weighted selection strategy with blacklist filtering
    fn weighted_select(
        &self,
        health_map: &HashMap<String, SignerHealth>,
    ) -> Result<&SignerWithMetadata, KoraError> {
        let mut rng = rand::rng();

        // Calculate total weight of HEALTHY signers
        let healthy_total_weight: u32 = self
            .signers
            .iter()
            .filter(|s| self.is_signer_healthy_internal(&s.name, health_map))
            .map(|s| s.weight)
            .sum();

        if healthy_total_weight > 0 {
            let mut target = rng.random_range(0..healthy_total_weight);
            for signer in &self.signers {
                if self.is_signer_healthy_internal(&signer.name, health_map) {
                    if target < signer.weight {
                        return Ok(signer);
                    }
                    target -= signer.weight;
                }
            }
        }

        // Fallback if no healthy signers or calculation failed
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
            failover_enabled: false,
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
            failover_enabled: false,
        };

        assert!(pool.get_next_signer().is_err());
        assert!(pool.is_empty());
        assert_eq!(pool.len(), 0);
    }

    #[test]
    fn test_failover_skips_blacklisted_signer() {
        let mut pool = create_test_pool();
        pool.failover_enabled = true;

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
        let mut pool = create_test_pool();
        pool.failover_enabled = true;
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
            failover_enabled: false,
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
