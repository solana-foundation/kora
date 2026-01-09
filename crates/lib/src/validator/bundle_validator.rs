use crate::{
    bundle::{constant::JITO_MAX_BUNDLE_SIZE, BundleError, JitoError},
    KoraError,
};

pub struct BundleValidator {}

impl BundleValidator {
    pub fn validate_jito_bundle_size(transactions: &[String]) -> Result<(), KoraError> {
        if transactions.is_empty() {
            return Err(BundleError::Empty.into());
        }
        if transactions.len() > JITO_MAX_BUNDLE_SIZE {
            return Err(BundleError::Jito(JitoError::BundleTooLarge(JITO_MAX_BUNDLE_SIZE)).into());
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_jito_bundle_size_empty() {
        let result = BundleValidator::validate_jito_bundle_size(&[]);
        assert!(result.is_err());
    }
    #[test]
    fn test_validate_jito_bundle_size_too_large() {
        let result = BundleValidator::validate_jito_bundle_size(&vec!["tx".to_string(); 6]);
        assert!(result.is_err());
    }
    #[test]
    fn test_validate_jito_bundle_size_valid() {
        let result = BundleValidator::validate_jito_bundle_size(&vec!["tx".to_string(); 5]);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_jito_bundle_size_boundary() {
        // Test boundary values: 1, 4, 5 should pass; 6, 7 should fail
        assert!(BundleValidator::validate_jito_bundle_size(&vec!["tx".to_string(); 1]).is_ok());
        assert!(BundleValidator::validate_jito_bundle_size(&vec!["tx".to_string(); 4]).is_ok());
        assert!(BundleValidator::validate_jito_bundle_size(&vec!["tx".to_string(); 5]).is_ok());
        assert!(BundleValidator::validate_jito_bundle_size(&vec!["tx".to_string(); 6]).is_err());
        assert!(BundleValidator::validate_jito_bundle_size(&vec!["tx".to_string(); 7]).is_err());
    }

    #[test]
    fn test_validate_jito_bundle_size_error_types() {
        // Empty returns BundleError::Empty
        let empty_err = BundleValidator::validate_jito_bundle_size(&[]).unwrap_err();
        assert!(matches!(empty_err, KoraError::InvalidTransaction(_)));

        // Too large returns JitoError::BundleTooLarge
        let large_err =
            BundleValidator::validate_jito_bundle_size(&vec!["tx".to_string(); 6]).unwrap_err();
        assert!(matches!(large_err, KoraError::JitoError(_)));
    }
}
