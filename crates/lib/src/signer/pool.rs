use crate::{
    error::KoraError,
    signer::config::{SelectionStrategy, SignerConfig, SignerPoolConfig},
};
use rand::Rng;
use solana_sdk::pubkey::Pubkey;
use solana_signers::{Signer, SolanaSigner};
use std::{
    str::FromStr,
    sync::{
        atomic::{AtomicU64, AtomicUsize, Ordering},
        Arc,
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

/// A pool of signers with different selection strategies
pub struct SignerPool {
    /// List of signers with their metadata
    signers: Vec<SignerWithMetadata>,
    /// Strategy for selecting signers
    strategy: SelectionStrategy,
    /// Current index for round-robin selection
    current_index: AtomicUsize,
    /// Total weight of all signers in the pool
    total_weight: u32,
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
        })
    }

    /// Get the next signer according to the configured strategy
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
            solana_signers::Signer::from_memory(&keypair1.to_base58_string()).unwrap();
        let external_signer2 =
            solana_signers::Signer::from_memory(&keypair2.to_base58_string()).unwrap();

        SignerPool {
            signers: vec![
                SignerWithMetadata::new("signer_1".to_string(), Arc::new(external_signer1), 1),
                SignerWithMetadata::new("signer_2".to_string(), Arc::new(external_signer2), 2),
            ],
            strategy: SelectionStrategy::RoundRobin,
            current_index: AtomicUsize::new(0),
            total_weight: 3,
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
        };

        assert!(pool.get_next_signer().is_err());
        assert!(pool.is_empty());
        assert_eq!(pool.len(), 0);
    }
}
