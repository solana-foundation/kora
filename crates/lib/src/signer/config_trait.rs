use crate::{error::KoraError, signer::KoraSigner};

/// Trait for signer configuration validation and building
pub trait SignerConfigTrait {
    /// Configuration type for this signer
    type Config;

    /// Validate the configuration before building the signer
    fn validate_config(config: &Self::Config, signer_name: &str) -> Result<(), KoraError>;

    /// Build a signer instance from the validated configuration
    fn build_from_config(config: &Self::Config, signer_name: &str)
        -> Result<KoraSigner, KoraError>;
}
