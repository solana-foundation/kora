use warp::Filter;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
struct TransferRequest {
    from: String,
    to: String,
    amount: u64,
}

#[derive(Debug, Serialize)]
struct TransferResponse {
    signed_tx: String,
}

pub fn transfer_route() -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
    warp::path("transferTransaction")
        .and(warp::post())
        .and(warp::body::json())
        .map(|req: TransferRequest| {
            let tx = crate::lib::transfer::create_transfer(&req.from, &req.to, req.amount);
            warp::reply::json(&TransferResponse { signed_tx: tx })
        })
}
