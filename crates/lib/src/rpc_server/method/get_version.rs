use crate::KoraError;
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct GetVersionResponse {
    pub version: String,
}

pub async fn get_version() -> Result<GetVersionResponse, KoraError> {
    Ok(GetVersionResponse { version: env!("CARGO_PKG_VERSION").to_string() })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_get_version_success() {
        let result = get_version().await;
        assert!(result.is_ok());
        let response = result.unwrap();
        assert!(!response.version.is_empty());
        assert_eq!(response.version, env!("CARGO_PKG_VERSION"));
    }
}
