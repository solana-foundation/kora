use crate::error::KoraError;

pub struct MockUsageTracker;

impl MockUsageTracker {
    pub async fn init_usage_limiter() -> Result<(), KoraError> {
        Ok(())
    }
}
