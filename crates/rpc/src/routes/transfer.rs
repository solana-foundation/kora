use axum::{Json, Router, routing::post};
use lib::actions::transfer::{TransferTransactionRequest, TransferTransactionResponse, handle_transfer_transaction};

pub fn routes() -> Router {
    Router::new().route(
        "/transferTransaction",
        post(transfer_transaction_handler),
    )
}

async fn transfer_transaction_handler(
    Json(req): Json<TransferTransactionRequest>,
) -> Json<TransferTransactionResponse> {
    let result = handle_transfer_transaction(req).await.unwrap();
    Json(result)
}

