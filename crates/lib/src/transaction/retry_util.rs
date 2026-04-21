use crate::KoraError;
use solana_sdk::signature::Signature;
use std::{future::Future, time::Duration};
use tokio::time::timeout;

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

pub(crate) fn signing_retry_window(sign_timeout: Duration, max_retries: u32) -> Duration {
    let per_attempt_timeout_ms = u64::try_from(sign_timeout.as_millis()).unwrap_or(u64::MAX);
    let total_attempts = u64::from(max_retries).saturating_add(1);
    let total_timeout_ms = per_attempt_timeout_ms.saturating_mul(total_attempts);
    let total_backoff_ms = (1..=max_retries)
        .fold(0u64, |total, attempt| total.saturating_add(signing_retry_backoff_ms(attempt)));

    Duration::from_millis(total_timeout_ms.saturating_add(total_backoff_ms))
}

pub(crate) async fn sign_with_retry<F, Fut>(
    sign_timeout: Duration,
    max_retries: u32,
    retry_operation_name: &str,
    error_prefix: &str,
    mut sign_attempt: F,
) -> Result<Signature, KoraError>
where
    F: FnMut() -> Fut,
    Fut: Future<Output = Result<Signature, KoraError>>,
{
    let total_attempts = max_retries + 1;
    let mut last_error: Option<KoraError> = None;

    for attempt in 0..=max_retries {
        if attempt > 0 {
            let backoff_ms = signing_retry_backoff_ms(attempt);
            log::warn!(
                "Retrying {} (attempt {}/{}). Backoff: {}ms",
                retry_operation_name,
                attempt,
                max_retries,
                backoff_ms
            );
            tokio::time::sleep(Duration::from_millis(backoff_ms)).await;
        }

        match timeout(sign_timeout, sign_attempt()).await {
            Ok(Ok(sig)) => return Ok(sig),
            Ok(Err(err)) => {
                log::error!(
                    "{} failed (attempt {}/{}): {}",
                    error_prefix,
                    attempt + 1,
                    total_attempts,
                    err
                );
                last_error = Some(err);
            }
            Err(_) => {
                let err_msg =
                    format!("{} timed out after {}s", error_prefix, sign_timeout.as_secs());
                log::error!("{} (attempt {}/{})", err_msg, attempt + 1, total_attempts);
                last_error = Some(KoraError::SigningError(err_msg));
            }
        }
    }

    Err(last_error.unwrap_or_else(|| {
        KoraError::SigningError(format!("{} failed after retries", error_prefix))
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    };
    use tokio::time::Duration;

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

    #[test]
    fn test_signing_retry_window_matches_default_signing_budget() {
        assert_eq!(signing_retry_window(Duration::from_secs(10), 2), Duration::from_millis(30_300));
    }

    #[test]
    fn test_signing_retry_window_grows_beyond_legacy_60_second_lease() {
        assert_eq!(signing_retry_window(Duration::from_secs(15), 5), Duration::from_millis(93_100));
    }

    #[tokio::test]
    async fn test_sign_with_retry_succeeds_after_retries() {
        let calls = Arc::new(AtomicUsize::new(0));
        let calls_for_closure = Arc::clone(&calls);

        let result = sign_with_retry(Duration::from_secs(1), 2, "signing", "Signing", move || {
            let calls = Arc::clone(&calls_for_closure);
            async move {
                if calls.fetch_add(1, Ordering::Relaxed) < 2 {
                    Err(KoraError::SigningError("temporary".to_string()))
                } else {
                    Ok(Signature::new_unique())
                }
            }
        })
        .await;

        assert!(result.is_ok());
        assert_eq!(calls.load(Ordering::Relaxed), 3);
    }

    #[tokio::test]
    async fn test_sign_with_retry_returns_last_error_after_exhaustion() {
        let result = sign_with_retry(Duration::from_secs(1), 1, "signing", "Signing", || async {
            Err(KoraError::SigningError("permanent".to_string()))
        })
        .await;

        assert!(result.is_err());
        match result {
            Err(KoraError::SigningError(msg)) => assert_eq!(msg, "permanent"),
            _ => panic!("Expected signing error"),
        }
    }
}
