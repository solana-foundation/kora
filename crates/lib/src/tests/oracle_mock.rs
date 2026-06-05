use std::sync::Arc;

thread_local! {
    pub static TEST_ORACLE: std::cell::RefCell<Option<Arc<dyn crate::oracle::PriceOracle + Send + Sync>>> = std::cell::RefCell::new(None);
}

pub fn get_price_oracle(
    _source: crate::oracle::PriceSource,
) -> Result<Arc<dyn crate::oracle::PriceOracle + Send + Sync>, crate::error::KoraError> {
    TEST_ORACLE.with(|o| {
        o.borrow().clone().ok_or_else(|| {
            crate::error::KoraError::InternalServerError("No test oracle set".to_string())
        })
    })
}
