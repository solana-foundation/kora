use crate::KoraError;

pub fn validate_division(divisor: f64) -> Result<(), KoraError> {
    if !divisor.is_finite() || divisor <= 0.0 {
        return Err(KoraError::RpcError(format!("Invalid division: {}", divisor)));
    }

    Ok(())
}
