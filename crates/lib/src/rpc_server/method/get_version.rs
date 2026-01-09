use crate::KoraError;
use serde::Serialize;
use utoipa::ToSchema;

#[derive(Debug, Clone, Serialize, ToSchema)]
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
        // Version should match Cargo.toml version
        assert_eq!(response.version, env!("CARGO_PKG_VERSION"));
    }
}
