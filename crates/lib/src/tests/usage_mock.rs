use crate::{error::KoraError, usage_limit};

pub struct UsageTracker;

impl UsageTracker {
    pub async fn init_usage_limiter() -> Result<(), KoraError> {
        let _ = usage_limit::UsageTracker::init_usage_limiter().await;
        Ok(())
    }
}
