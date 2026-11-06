use crate::{error::KoraError, usage_limit};

pub struct UsageTracker;

impl UsageTracker {
    pub async fn init_usage_limiter() -> Result<(), KoraError> {
        match usage_limit::UsageTracker::init_usage_limiter().await {
            Err(KoraError::InternalServerError(ref msg)) if msg.contains("already initialized") => {
                log::debug!(
                    "Usage limiter already initialized in test environment, reusing existing instance"
                );
                Ok(())
            }
            res => res,
        }
    }
}
