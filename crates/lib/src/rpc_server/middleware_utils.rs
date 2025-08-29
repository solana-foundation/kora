use futures_util::TryStreamExt;
use http::Request;
use jsonrpsee::server::logger::Body;

pub fn default_sig_verify() -> bool {
    false
}

pub async fn extract_parts_and_body_bytes(
    request: Request<Body>,
) -> (http::request::Parts, Vec<u8>) {
    let (parts, body) = request.into_parts();
    let body_bytes = body
        .try_fold(Vec::new(), |mut acc, chunk| async move {
            acc.extend_from_slice(&chunk);
            Ok(acc)
        })
        .await
        .unwrap_or_default();
    (parts, body_bytes)
}

pub fn get_jsonrpc_method(body_bytes: &[u8]) -> Option<String> {
    match serde_json::from_slice::<serde_json::Value>(body_bytes) {
        Ok(val) => val.get("method").and_then(|m| m.as_str()).map(|s| s.to_string()),
        Err(_) => None,
    }
}
