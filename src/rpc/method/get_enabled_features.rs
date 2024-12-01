use serde::{Deserialize, Serialize};

use crate::common::{Feature, KoraError};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetEnabledFeaturesResponse {
    pub features: Vec<String>,
}

pub async fn get_enabled_features(
    features: &[Feature],
) -> Result<GetEnabledFeaturesResponse, KoraError> {
    let response = GetEnabledFeaturesResponse {
        features: features
            .iter()
            .map(|f| match f {
                Feature::Gasless => "gasless".to_string(),
            })
            .collect(),
    };

    Ok(response)
}
