use reqwest::Client;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug)]
pub struct TurnkeySigner {
    pub organization_id: String,
    pub private_key_id: String,
    pub api_public_key: String,
    pub api_private_key: String,
    pub public_key: String,
    pub api_base_url: String,
    pub client: Client,
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct SignRequest {
    #[serde(rename = "type")]
    pub activity_type: String,
    pub timestamp_ms: String,
    pub organization_id: String,
    pub parameters: SignParameters,
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct SignParameters {
    pub sign_with: String,
    pub payload: String,
    pub encoding: String,
    pub hash_function: String,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ActivityResponse {
    pub activity: Activity,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Activity {
    pub result: Option<ActivityResult>,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ActivityResult {
    pub sign_raw_payload_result: Option<SignResult>,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct SignResult {
    pub r: String,
    pub s: String,
}
