use mockito::{Mock, ServerGuard};
use serde_json::json;

pub async fn mock_get_payer_signer(server: &mut ServerGuard, pubkey: &str) -> Mock {
    server
        .mock("POST", "/")
        .match_body(mockito::Matcher::PartialJson(json!({
            "method": "getPayerSigner"
        })))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            json!({
                "result": {
                    "signer_address": pubkey
                }
            })
            .to_string(),
        )
        .create_async()
        .await
}

pub async fn mock_sign_and_send_success(server: &mut ServerGuard, signature: &str) -> Mock {
    server
        .mock("POST", "/")
        .match_body(mockito::Matcher::PartialJson(json!({
            "method": "signAndSendTransaction"
        })))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            json!({
                "result": {
                    "signature": signature
                }
            })
            .to_string(),
        )
        .create_async()
        .await
}

pub async fn mock_sign_and_send_error(
    server: &mut ServerGuard,
    error_code: i32,
    error_msg: &str,
) -> Mock {
    server
        .mock("POST", "/")
        .match_body(mockito::Matcher::PartialJson(json!({
            "method": "signAndSendTransaction"
        })))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            json!({
                "error": {
                    "code": error_code,
                    "message": error_msg
                }
            })
            .to_string(),
        )
        .create_async()
        .await
}
