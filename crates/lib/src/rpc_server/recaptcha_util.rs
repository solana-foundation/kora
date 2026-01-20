use crate::{
    constant::{RECAPTCHA_TIMEOUT_SECS, RECAPTCHA_VERIFY_URL},
    error::KoraError,
    rpc_server::middleware_utils::build_response_with_graceful_error,
    sanitize_error,
};
use http::{Response, StatusCode};
use jsonrpsee::server::logger::Body;
use reqwest::Client;
use serde::Deserialize;
use std::time::Duration;

#[derive(Debug, Deserialize)]
struct RecaptchaVerifyResponse {
    success: bool,
    score: Option<f64>,
    #[serde(rename = "error-codes")]
    error_codes: Option<Vec<String>>,
}

#[derive(Clone)]
pub struct RecaptchaConfig {
    pub secret: String,
    pub score_threshold: f64,
    pub protected_methods: Vec<String>,
}

impl RecaptchaConfig {
    pub fn new(secret: String, score_threshold: f64, protected_methods: Vec<String>) -> Self {
        Self { secret, score_threshold, protected_methods }
    }

    pub fn is_protected_method(&self, method: &str) -> bool {
        self.protected_methods.iter().any(|m| m == method)
    }

    pub async fn validate(&self, token: Option<&str>, method: &str) -> Result<(), Response<Body>> {
        if !self.is_protected_method(method) {
            return Ok(());
        }

        let token = match token {
            Some(t) if !t.is_empty() => t,
            _ => {
                return Err(build_response_with_graceful_error(None, StatusCode::UNAUTHORIZED, ""))
            }
        };

        if let Err(e) = self.verify_token(token).await {
            log::error!("reCAPTCHA verification error: {}", sanitize_error!(e));
            return Err(build_response_with_graceful_error(None, StatusCode::UNAUTHORIZED, ""));
        }

        Ok(())
    }

    async fn verify_token(&self, token: &str) -> Result<f64, KoraError> {
        let client = Client::builder()
            .timeout(Duration::from_secs(RECAPTCHA_TIMEOUT_SECS))
            .build()
            .map_err(|e| {
                KoraError::RecaptchaError(format!(
                    "Failed to create HTTP client: {}",
                    sanitize_error!(e)
                ))
            })?;

        let response = client
            .post(RECAPTCHA_VERIFY_URL)
            .form(&[("secret", &self.secret), ("response", &token.to_string())])
            .send()
            .await
            .map_err(|e| {
                KoraError::RecaptchaError(format!("API call failed: {}", sanitize_error!(e)))
            })?;

        if !response.status().is_success() {
            let status = response.status();
            #[cfg(feature = "unsafe-debug")]
            log::error!("reCAPTCHA API returned error status: {}", status);
            #[cfg(not(feature = "unsafe-debug"))]
            log::error!("reCAPTCHA API returned error status: {}", status.as_u16());

            return Err(KoraError::RecaptchaError(format!(
                "API returned status: {}",
                status.as_u16()
            )));
        }

        let verify_response: RecaptchaVerifyResponse = response.json().await.map_err(|e| {
            KoraError::RecaptchaError(format!("Failed to parse response: {}", sanitize_error!(e)))
        })?;

        if !verify_response.success {
            let errors = verify_response.error_codes.unwrap_or_default().join(", ");
            return Err(KoraError::RecaptchaError(format!(
                "Verification failed: {}",
                sanitize_error!(errors)
            )));
        }

        let score = verify_response
            .score
            .ok_or_else(|| KoraError::RecaptchaError("Response missing score".to_string()))?;

        if score < self.score_threshold {
            return Err(KoraError::RecaptchaError(format!(
                "Score {:.2} below threshold {:.2}",
                score, self.score_threshold
            )));
        }

        Ok(score)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_recaptcha_config_is_protected_method() {
        let config = RecaptchaConfig::new(
            "secret".to_string(),
            0.5,
            vec!["signTransaction".to_string(), "signAndSendTransaction".to_string()],
        );

        assert!(config.is_protected_method("signTransaction"));
        assert!(config.is_protected_method("signAndSendTransaction"));
        assert!(!config.is_protected_method("getConfig"));
        assert!(!config.is_protected_method("liveness"));
    }

    #[test]
    fn test_optional_recaptcha_config() {
        let no_config: Option<RecaptchaConfig> = None;
        assert!(no_config.is_none());

        let with_config = Some(RecaptchaConfig::new(
            "secret".to_string(),
            0.5,
            vec!["signTransaction".to_string()],
        ));
        assert!(with_config.is_some());
    }
}
