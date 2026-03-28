use crate::{
    error::KoraError,
    signer::config::{SelectionStrategy, SignerConfig, SignerPoolConfig},
};
use rand::Rng;
use solana_keychain::{Signer, SolanaSigner};
use solana_sdk::pubkey::Pubkey;
use std::{
    str::FromStr,
    sync::{
        atomic::{AtomicU64, AtomicUsize, Ordering},
        Arc, Mutex,
    },
};

const DEFAULT_WEIGHT: u32 = 1;

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
    /// Tracks (consecutive_failures, is_healthy) thread-safely
    health: Mutex<(u32, bool)>,
    /// Timestamp of when the signer last failed to probe for recovery
    last_failed_at: Mutex<Option<std::time::Instant>>,
}

impl Clone for SignerWithMetadata {
    fn clone(&self) -> Self {
        let health = *self.health.lock().unwrap();
        let last_failed_at = *self.last_failed_at.lock().unwrap();

        Self {
            name: self.name.clone(),
            signer: self.signer.clone(),
            weight: self.weight,
            last_used: AtomicU64::new(self.last_used.load(Ordering::Relaxed)),
            health: Mutex::new(health),
            last_failed_at: Mutex::new(last_failed_at),
        }
    }
}

impl SignerWithMetadata {
    /// Number of consecutive failures before marking the signer as unhealthy
    const MAX_CONSECUTIVE_FAILURES: u32 = 3;
    /// Seconds to wait before allowing an unhealthy signer to be probed for recovery
    const RECOVERY_PROBE_SECS: u64 = 30;

    /// Create a new signer with metadata
    pub(crate) fn new(name: String, signer: Arc<Signer>, weight: u32) -> Self {
        Self {
            name,
            signer,
            weight,
            last_used: AtomicU64::new(0),
            health: Mutex::new((0, true)),
            last_failed_at: Mutex::new(None),
        }
    }

    /// Update the last used timestamp to current time
    fn update_last_used(&self) {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        self.last_used.store(now, Ordering::Relaxed);
    }

    /// Records a successful signature creation, resetting consecutive failures to 0
    pub(crate) fn record_success(&self) {
        let mut health = self.health.lock().unwrap();
        *health = (0, true);
        *self.last_failed_at.lock().unwrap() = None;
    }

    /// Records a failed signature attempt, potentially degrading the signer's health
    pub(crate) fn record_failure(&self) {
        let mut health = self.health.lock().unwrap();
        health.0 += 1;
        if health.0 >= Self::MAX_CONSECUTIVE_FAILURES {
            if health.1 {
                log::warn!(
                    "Signer '{}' marked unhealthy after {} consecutive failures",
                    self.name,
                    health.0
                );
            } else {
                log::debug!("Recovery probe failed for signer '{}', resetting cooldown", self.name);
            }
            health.1 = false;
            *self.last_failed_at.lock().unwrap() = Some(std::time::Instant::now());
        }
    }

    /// Whether the signer is currently eligible for pool selection
    #[allow(dead_code)]
    pub(crate) fn is_healthy(&self) -> bool {
        self.health.lock().unwrap().1
    }

    #[allow(dead_code)]
    pub(crate) fn name(&self) -> &str {
        &self.name
    }
}

/// A pool of signers with different selection strategies
pub struct SignerPool {
    /// List of signers with their metadata
    signers: Vec<SignerWithMetadata>,
    /// Strategy for selecting signers
    strategy: SelectionStrategy,
    /// Current index for round-robin selection
    current_index: AtomicUsize,
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
        Self {
            signers,
            strategy: SelectionStrategy::RoundRobin,
            current_index: AtomicUsize::new(0),
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
        })
    }

    /// Records a successful signature creation, resetting consecutive failures to 0
    pub fn record_signing_success(&self, signer: &Arc<Signer>) {
        if let Some(meta) = self.signers.iter().find(|s| Arc::ptr_eq(&s.signer, signer)) {
            meta.record_success();
        }
    }

    /// Records a failed signature attempt, potentially degrading the signer's health
    pub fn record_signing_failure(&self, signer: &Arc<Signer>) {
        if let Some(meta) = self.signers.iter().find(|s| Arc::ptr_eq(&s.signer, signer)) {
            meta.record_failure();
        }
    }

    /// Filters the active signers down to only those that haven't hit the failure limit.
    /// Falls back to the entire pool if every signer is marked unhealthy.
    fn healthy_signers(&self) -> Vec<&SignerWithMetadata> {
        let healthy: Vec<_> = self
            .signers
            .iter()
            .filter(|s| {
                let h = s.health.lock().unwrap();
                let is_healthy = h.1;
                if is_healthy {
                    return true;
                }

                // Check if enough time passed for recovery probe. If 30 seconds have elapsed,
                // temporarily treat the signer as healthy to attempt a background recovery check.
                if let Some(last_failed) = *s.last_failed_at.lock().unwrap() {
                    if last_failed.elapsed().as_secs() >= SignerWithMetadata::RECOVERY_PROBE_SECS {
                        log::debug!("Probing recovery for signer '{}'", s.name());
                        true
                    } else {
                        false
                    }
                } else {
                    false
                }
            })
            .collect();

        if healthy.is_empty() {
            log::error!(
                "All {} signers are unhealthy! Falling back to full pool",
                self.signers.len()
            );
            self.signers.iter().collect()
        } else {
            healthy
        }
    }

    /// Get the next signer according to the configured strategy
    pub fn get_next_signer(&self) -> Result<Arc<Signer>, KoraError> {
        if self.signers.is_empty() {
            return Err(KoraError::InternalServerError("Signer pool is empty".to_string()));
        }

        let healthy = self.healthy_signers();

        let signer_meta = match self.strategy {
            SelectionStrategy::RoundRobin => self.round_robin_select_from(&healthy),
            SelectionStrategy::Random => self.random_select_from(&healthy),
            SelectionStrategy::Weighted => self.weighted_select_from(&healthy),
        }?;

        signer_meta.update_last_used();
        Ok(Arc::clone(&signer_meta.signer))
    }

    fn round_robin_select_from<'a>(
        &self,
        signers: &[&'a SignerWithMetadata],
    ) -> Result<&'a SignerWithMetadata, KoraError> {
        let index = self.current_index.fetch_add(1, Ordering::AcqRel);
        Ok(signers[index % signers.len()])
    }

    fn random_select_from<'a>(
        &self,
        signers: &[&'a SignerWithMetadata],
    ) -> Result<&'a SignerWithMetadata, KoraError> {
        let mut rng = rand::rng();
        let index = rng.random_range(0..signers.len());
        Ok(signers[index])
    }

    fn weighted_select_from<'a>(
        &self,
        signers: &[&'a SignerWithMetadata],
    ) -> Result<&'a SignerWithMetadata, KoraError> {
        let total: u32 = signers.iter().map(|s| s.weight).sum();
        let mut rng = rand::rng();
        let mut target = rng.random_range(0..total.max(1));
        for signer in signers {
            if target < signer.weight {
                return Ok(signer);
            }
            target -= signer.weight;
        }
        Ok(signers[0])
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

        // Allow pinned signers to participate in the 30-second recovery probe,
        // maintaining consistency with normal pool auto-selection behavior.
        let is_eligible = if signer_meta.is_healthy() {
            true
        } else if let Some(last_failed) = *signer_meta.last_failed_at.lock().unwrap() {
            last_failed.elapsed().as_secs() >= SignerWithMetadata::RECOVERY_PROBE_SECS
        } else {
            false
        };

        if !is_eligible {
            return Err(KoraError::ValidationError(format!(
                "Pinned signer {} is unhealthy (recovery probe not yet eligible after cooldown)",
                pubkey
            )));
        }

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
        };

        assert!(pool.get_next_signer().is_err());
        assert!(pool.is_empty());
        assert_eq!(pool.len(), 0);
    }

    #[test]
    fn test_signer_marked_unhealthy_after_3_failures() {
        let pool = create_test_pool();
        let meta = &pool.signers[0];

        assert!(meta.is_healthy());

        meta.record_failure();
        assert!(meta.is_healthy()); // still healthy after 1

        meta.record_failure();
        assert!(meta.is_healthy()); // still healthy after 2

        meta.record_failure();
        assert!(!meta.is_healthy()); // unhealthy after 3
    }

    #[test]
    fn test_recovery_after_success() {
        let pool = create_test_pool();
        let meta = &pool.signers[0];

        // Mark unhealthy
        meta.record_failure();
        meta.record_failure();
        meta.record_failure();
        assert!(!meta.is_healthy());

        // One success recovers it
        meta.record_success();
        assert!(meta.is_healthy());
    }

    #[test]
    fn test_healthy_signers_filters_unhealthy() {
        let pool = create_test_pool();

        // Mark first signer unhealthy
        pool.signers[0].record_failure();
        pool.signers[0].record_failure();
        pool.signers[0].record_failure();

        let healthy = pool.healthy_signers();
        assert_eq!(healthy.len(), 1);
        assert_eq!(healthy[0].name(), "signer_2");
    }

    #[test]
    fn test_fallback_when_all_signers_unhealthy() {
        let pool = create_test_pool();

        // Mark ALL signers unhealthy
        for signer in &pool.signers {
            signer.record_failure();
            signer.record_failure();
            signer.record_failure();
        }

        // healthy_signers should fallback to full pool
        let healthy = pool.healthy_signers();
        assert_eq!(healthy.len(), 2); // returns all as fallback

        // get_next_signer should still work
        assert!(pool.get_next_signer().is_ok());
    }

    #[test]
    fn test_round_robin_skips_unhealthy() {
        let pool = create_test_pool();

        // Mark signer_1 unhealthy
        pool.signers[0].record_failure();
        pool.signers[0].record_failure();
        pool.signers[0].record_failure();

        // All selections should return signer_2
        let signer2_pubkey = pool.signers[1].signer.pubkey().to_string();
        for _ in 0..10 {
            let selected = pool.get_next_signer().unwrap();
            assert_eq!(selected.pubkey().to_string(), signer2_pubkey);
        }
    }

    #[test]
    fn test_recovery_probe_after_30_seconds() {
        let pool = create_test_pool();
        let meta = &pool.signers[0];

        // Mark signer_1 unhealthy by forcing 3 failures
        meta.record_failure();
        meta.record_failure();
        meta.record_failure();
        assert!(!meta.is_healthy());

        // Immediately checking healthy signers should exclude signer_1
        let healthy_before_time = pool.healthy_signers();
        assert_eq!(healthy_before_time.len(), 1);
        assert_eq!(healthy_before_time[0].name(), "signer_2");

        // Simulate 31 seconds passing by tricking the last_failed_at timestamp
        *meta.last_failed_at.lock().unwrap() =
            Some(std::time::Instant::now() - std::time::Duration::from_secs(31));

        // Now healthy_signers() should tentatively ALLOW signer_1 back into the rotation
        // to probe for recovery, even though is_healthy() is strictly still false.
        let healthy_after_time = pool.healthy_signers();
        assert_eq!(healthy_after_time.len(), 2); // Both signers included!

        // Verify pinned path also allows after 30s mock
        let pinned_signer = pool.get_signer_by_pubkey(&meta.signer.pubkey().to_string());
        assert!(pinned_signer.is_ok());

        // Simulate selection and successful record_success
        meta.record_success();
        assert!(meta.is_healthy()); // Permanently healthy again
    }
}
