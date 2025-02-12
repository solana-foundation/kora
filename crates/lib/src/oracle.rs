use crate::{constant::JUPITER_API_URL, error::KoraError};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JupiterPriceResponse {
    pub data: Vec<JupiterPriceData>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JupiterPriceData {
    #[serde(rename = "id")]
    pub mint: String,
    #[serde(rename = "mintSymbol")]
    pub symbol: String,
    #[serde(rename = "price")]
    pub price: f64,
}


pub async fn get_token_price(mint_address: &str) -> Result<f64, KoraError> {
  
    let client = reqwest::Client::new();

    let jupiter_url = format!("{}/price?ids={}", JUPITER_API_URL, mint_address);
    let response = client.get(&jupiter_url).send().await;

    let price = match response {
        Ok(response) => {
            if response.status().is_success() {
                let jupiter_response: JupiterPriceResponse =
                    response.json().await.map_err(|e| {
                        KoraError::InternalServerError(format!(
                            "Failed to parse Jupiter API response: {}",
                            e
                        ))
                    })?;

                     //check price data not empty
                     if jupiter_response.data.is_empty() {
                        return Err(KoraError::InternalServerError(format!(
                            "Jupiter API returned empty price data for mint: {}",
                            mint_address
                        )));
                    }

                  let price_data =  jupiter_response
                  .data
                  .first()
                  .ok_or(KoraError::InternalServerError(format!("No price data returned from Jupiter API")))?;

                    price_data.price
            } else {
                return Err(KoraError::InternalServerError(format!(
                    "Jupiter price API returned error {}: {}",
                    response.status(), mint_address
                )));
            }
        }
        Err(e) => {
            return Err(KoraError::InternalServerError(format!(
                "Failed to fetch price from Jupiter API: {}",
                e
            )));
        }
    };

    Ok(price)
}
