use serde::{Deserialize, Serialize};

use super::error::KoraError;

#[derive(Debug, Serialize, Deserialize)]
pub struct KoraResponse<T> {
    success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    data: Option<T>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<KoraError>,
}

impl<T> KoraResponse<T> {
    pub fn ok(data: T) -> Self {
        KoraResponse {
            success: true,
            data: Some(data),
            error: None,
        }
    }

    pub fn err(error: KoraError) -> Self {
        KoraResponse {
            success: false,
            data: None,
            error: Some(error),
        }
    }
}