const SIGNING_RETRY_BACKOFF_BASE_MS: u64 = 100;
const SIGNING_RETRY_BACKOFF_MAX_EXPONENT: u32 = 7;

/// Returns exponential backoff in milliseconds for retry attempt `attempt`.
///
/// Attempt 0 (initial try) has no backoff.
/// Attempts 1+ use: `100ms * 2^(attempt-1)` capped at exponent 7 (~12.8s).
pub(crate) fn signing_retry_backoff_ms(attempt: u32) -> u64 {
    if attempt == 0 {
        return 0;
    }

    let exponent = (attempt - 1).min(SIGNING_RETRY_BACKOFF_MAX_EXPONENT);
    SIGNING_RETRY_BACKOFF_BASE_MS * 2u64.pow(exponent)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_signing_retry_backoff_ms_attempt_zero() {
        assert_eq!(signing_retry_backoff_ms(0), 0);
    }

    #[test]
    fn test_signing_retry_backoff_ms_exponential_growth() {
        assert_eq!(signing_retry_backoff_ms(1), 100);
        assert_eq!(signing_retry_backoff_ms(2), 200);
        assert_eq!(signing_retry_backoff_ms(3), 400);
    }

    #[test]
    fn test_signing_retry_backoff_ms_is_capped() {
        assert_eq!(signing_retry_backoff_ms(8), 12_800);
        assert_eq!(signing_retry_backoff_ms(20), 12_800);
    }
}
