use crate::{
    error::KoraError,
    signer::config::{SelectionStrategy, SignerConfig, SignerPoolConfig},
};
use parking_lot::Mutex;
use rand::RngExt;
use solana_keychain::{Signer, SolanaSigner};
use solana_sdk::pubkey::Pubkey;
use std::{
    str::FromStr,
    sync::{
        atomic::{AtomicU64, AtomicUsize, Ordering},
        Arc,
    },
};

const DEFAULT_WEIGHT: u32 = 1;

/// Metadata associated with a signer in the pool
#[derive(Debug, Clone, Copy)]
pub(crate) struct HealthState {
    pub(crate) consecutive_failures: u32,
    pub(crate) is_healthy: bool,
    pub(crate) last_failed_at: Option<std::time::Instant>,
    pub(crate) probe_in_flight: bool,
    pub(crate) probe_started_at: Option<std::time::Instant>,
}

impl Default for HealthState {
    fn default() -> Self {
        Self {
            consecutive_failures: 0,
            is_healthy: true,
            last_failed_at: None,
            probe_in_flight: false,
            probe_started_at: None,
        }
    }
}

pub(crate) struct SignerWithMetadata {
    /// Human-readable name for this signer
    name: String,
    /// The actual signer instance
    signer: Arc<Signer>,
    /// Weight for weighted selection (higher = more likely to be selected)
    weight: u32,
    /// Timestamp of last use (Unix timestamp in seconds)
    last_used: AtomicU64,
    /// Tracks health and failure state thread-safely in a single lock
    health: Mutex<HealthState>,
}

impl Clone for SignerWithMetadata {
    fn clone(&self) -> Self {
        let health = *self.health.lock();

        Self {
            name: self.name.clone(),
            signer: self.signer.clone(),
            weight: self.weight,
            last_used: AtomicU64::new(self.last_used.load(Ordering::Relaxed)),
            health: Mutex::new(health),
        }
    }
}

impl SignerWithMetadata {
    /// Number of consecutive failures before marking the signer as unhealthy
    const MAX_CONSECUTIVE_FAILURES: u32 = 3;
    /// Seconds to wait before allowing an unhealthy signer to be probed for recovery
    const RECOVERY_PROBE_SECS: u64 = 30;
    /// Seconds after which an in-flight probe lock is considered stale
    const PROBE_LEASE_SECS: u64 = 60;

    /// Create a new signer with metadata
    pub(crate) fn new(name: String, signer: Arc<Signer>, weight: u32) -> Self {
        Self {
            name,
            signer,
            weight,
            last_used: AtomicU64::new(0),
            health: Mutex::new(HealthState::default()),
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
        let mut health = self.health.lock();
        *health = HealthState::default();
    }

    /// Records a failed signature attempt, potentially degrading the signer's health
    pub(crate) fn record_failure(&self) {
        let mut health = self.health.lock();
        health.consecutive_failures += 1;
        if health.consecutive_failures >= Self::MAX_CONSECUTIVE_FAILURES {
            if health.is_healthy {
                log::warn!(
                    "Signer '{}' marked unhealthy after {} consecutive failures",
                    self.name,
                    health.consecutive_failures
                );
            } else {
                log::debug!("Recovery probe failed for signer '{}', resetting cooldown", self.name);
            }
            health.is_healthy = false;
            health.last_failed_at = Some(std::time::Instant::now());
            health.probe_in_flight = false;
            health.probe_started_at = None;
        }
    }

    #[cfg(test)]
    pub(crate) fn is_healthy(&self) -> bool {
        self.health.lock().is_healthy
    }

    pub(crate) fn name(&self) -> &str {
        &self.name
    }

    fn release_stale_probe_lock_if_needed(&self, health: &mut HealthState) {
        if !health.probe_in_flight {
            return;
        }

        match health.probe_started_at {
            Some(started_at) if started_at.elapsed().as_secs() >= Self::PROBE_LEASE_SECS => {
                log::warn!(
                    "Releasing stale probe lock for signer '{}' after {}s lease timeout",
                    self.name,
                    Self::PROBE_LEASE_SECS
                );
                health.probe_in_flight = false;
                health.probe_started_at = None;
            }
            None => {
                log::warn!(
                    "Signer '{}' had probe_in_flight=true without probe_started_at; clearing stale lock",
                    self.name
                );
                health.probe_in_flight = false;
            }
            _ => {}
        }
    }

    fn is_probe_eligible_with_lock(&self, health: &mut HealthState) -> bool {
        if health.is_healthy {
            return true;
        }

        let Some(last_failed) = health.last_failed_at else {
            return false;
        };

        if last_failed.elapsed().as_secs() < Self::RECOVERY_PROBE_SECS {
            return false;
        }

        self.release_stale_probe_lock_if_needed(health);
        !health.probe_in_flight
    }

    fn is_eligible_for_selection(&self) -> bool {
        let mut health = self.health.lock();
        self.is_probe_eligible_with_lock(&mut health)
    }

    fn try_acquire_probe_lock_if_needed(&self) -> bool {
        let mut health = self.health.lock();
        if !self.is_probe_eligible_with_lock(&mut health) {
            return false;
        }

        if health.is_healthy {
            return true;
        }

        log::debug!("Probing recovery for signer '{}'", self.name());
        health.probe_in_flight = true;
        health.probe_started_at = Some(std::time::Instant::now());
        true
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
        self.with_signer_metadata(signer, "record_signing_success", |meta| meta.record_success());
    }

    /// Records a failed signature attempt, potentially degrading the signer's health
    pub fn record_signing_failure(&self, signer: &Arc<Signer>) {
        self.with_signer_metadata(signer, "record_signing_failure", |meta| meta.record_failure());
    }

    fn with_signer_metadata<F>(&self, signer: &Arc<Signer>, context: &str, on_found: F)
    where
        F: FnOnce(&SignerWithMetadata),
    {
        if let Some(meta) = self.signers.iter().find(|s| Arc::ptr_eq(&s.signer, signer)) {
            on_found(meta);
        } else {
            log::warn!(
                "{context} called for signer {} not found in pool; health tracking skipped",
                signer.pubkey()
            );
        }
    }

    /// Filters the active signers down to healthy signers plus unhealthy signers whose
    /// recovery probe cooldown has elapsed and no probe lock is currently held.
    fn healthy_signers(&self) -> Result<Vec<&SignerWithMetadata>, KoraError> {
        let healthy: Vec<_> =
            self.signers.iter().filter(|s| s.is_eligible_for_selection()).collect();

        if healthy.is_empty() {
            log::error!(
                "No signer is currently eligible (all unhealthy and recovery cooldown/probe lock active) across {} signers",
                self.signers.len()
            );
            return Err(KoraError::InternalServerError(
                "No healthy signers available (all signers are unhealthy or in recovery cooldown)"
                    .to_string(),
            ));
        }

        Ok(healthy)
    }

    /// Get the next signer according to the configured strategy
    pub fn get_next_signer(&self) -> Result<Arc<Signer>, KoraError> {
        if self.signers.is_empty() {
            return Err(KoraError::InternalServerError("Signer pool is empty".to_string()));
        }

        // Retry selection a small bounded number of times to avoid transient
        // races where a signer becomes ineligible between filtering and probe lock acquisition.
        for _ in 0..self.signers.len().max(1) {
            let healthy = self.healthy_signers()?;

            let signer_meta = match self.strategy {
                SelectionStrategy::RoundRobin => self.round_robin_select_from(&healthy),
                SelectionStrategy::Random => self.random_select_from(&healthy),
                SelectionStrategy::Weighted => self.weighted_select_from(&healthy),
            }?;

            if !signer_meta.try_acquire_probe_lock_if_needed() {
                continue;
            }

            signer_meta.update_last_used();
            return Ok(Arc::clone(&signer_meta.signer));
        }

        Err(KoraError::InternalServerError(
            "No healthy signers available after probe lock contention".to_string(),
        ))
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

        if !signer_meta.try_acquire_probe_lock_if_needed() {
            return Err(KoraError::ValidationError(format!(
                "Pinned signer {} is unhealthy or currently unavailable for recovery probe",
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

        let healthy = pool.healthy_signers().unwrap();
        assert_eq!(healthy.len(), 1);
        assert_eq!(healthy[0].name(), "signer_2");
    }

    #[test]
    fn test_error_when_all_signers_unhealthy() {
        let pool = create_test_pool();

        // Mark ALL signers unhealthy
        for signer in &pool.signers {
            signer.record_failure();
            signer.record_failure();
            signer.record_failure();
        }

        let healthy = pool.healthy_signers();
        assert!(healthy.is_err());

        // get_next_signer should now fail-fast instead of routing to unhealthy signers.
        assert!(pool.get_next_signer().is_err());
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
        let healthy_before_time = pool.healthy_signers().unwrap();
        assert_eq!(healthy_before_time.len(), 1);
        assert_eq!(healthy_before_time[0].name(), "signer_2");

        // Simulate 31 seconds passing by tricking the last_failed_at timestamp
        meta.health.lock().last_failed_at =
            Some(std::time::Instant::now() - std::time::Duration::from_secs(31));

        // Now healthy_signers() should tentatively ALLOW signer_1 back into the rotation
        // to probe for recovery, even though is_healthy() is strictly still false.
        let healthy_after_time = pool.healthy_signers().unwrap();
        assert_eq!(healthy_after_time.len(), 2); // Both signers included!

        // Verify pinned path also allows after 30s mock
        let pinned_signer = pool.get_signer_by_pubkey(&meta.signer.pubkey().to_string());
        assert!(pinned_signer.is_ok());

        // Simulate selection and successful record_success
        meta.record_success();
        assert!(meta.is_healthy()); // Permanently healthy again
    }

    #[test]
    fn test_pinned_signer_unhealthy_within_cooldown_returns_error() {
        let pool = create_test_pool();
        let meta = &pool.signers[0];
        let pubkey = meta.signer.pubkey().to_string();

        meta.record_failure();
        meta.record_failure();
        meta.record_failure();
        assert!(!meta.is_healthy());

        // last_failed_at was just set — cooldown has not expired
        let result = pool.get_signer_by_pubkey(&pubkey);
        assert!(result.is_err());
        assert!(matches!(result.err().unwrap(), KoraError::ValidationError(_)));
    }

    #[test]
    fn test_pool_record_signing_failure_via_arc() {
        let pool = create_test_pool();

        // get_next_signer() returns Arc::clone of the internal Arc; ptr_eq should match it back
        let signer = pool.get_next_signer().unwrap();
        pool.record_signing_failure(&signer);
        pool.record_signing_failure(&signer);
        pool.record_signing_failure(&signer);

        assert!(!pool.signers[0].is_healthy());
        let healthy = pool.healthy_signers().unwrap();
        assert_eq!(healthy.len(), 1);
        assert_eq!(healthy[0].name(), "signer_2");
    }

    #[test]
    fn test_pinned_signer_recovery_probe_respects_in_flight_lock() {
        let pool = create_test_pool();
        let meta = &pool.signers[0];
        let pubkey = meta.signer.pubkey().to_string();

        meta.record_failure();
        meta.record_failure();
        meta.record_failure();
        assert!(!meta.is_healthy());

        // Make signer eligible for recovery probe.
        meta.health.lock().last_failed_at =
            Some(std::time::Instant::now() - std::time::Duration::from_secs(31));

        // First pinned request acquires the probe lock.
        assert!(pool.get_signer_by_pubkey(&pubkey).is_ok());

        // Second pinned request should be rejected while probe is in-flight.
        let second = pool.get_signer_by_pubkey(&pubkey);
        assert!(second.is_err());
        assert!(matches!(second.err().unwrap(), KoraError::ValidationError(_)));
    }

    #[test]
    fn test_stale_probe_lock_is_released_after_lease_timeout() {
        let pool = create_test_pool();
        let meta = &pool.signers[0];

        meta.record_failure();
        meta.record_failure();
        meta.record_failure();
        assert!(!meta.is_healthy());

        // Make signer eligible for recovery probe.
        meta.health.lock().last_failed_at =
            Some(std::time::Instant::now() - std::time::Duration::from_secs(31));

        // Acquire a probe lock, then simulate a stuck request by backdating probe_started_at.
        let healthy = pool.healthy_signers().unwrap();
        assert_eq!(healthy.len(), 2);
        assert!(pool.signers[0].try_acquire_probe_lock_if_needed());
        {
            let mut health = meta.health.lock();
            health.probe_started_at = Some(
                std::time::Instant::now()
                    - std::time::Duration::from_secs(SignerWithMetadata::PROBE_LEASE_SECS + 1),
            );
        }

        // Stale lock should be cleared automatically and signer should become selectable again.
        let healthy_after_lease = pool.healthy_signers().unwrap();
        assert_eq!(healthy_after_lease.len(), 2);
    }
}
