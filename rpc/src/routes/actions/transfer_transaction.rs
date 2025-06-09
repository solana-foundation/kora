use crate::context::Context;
use solana_sdk::{
    pubkey::Pubkey,
    hash::Hash,
    signature::Signature,
    transaction::VersionedTransaction
};
use serde::{Deserialize, Serialize};
use warp::{Rejection, Reply};

#[derive(Deserialize)]
pub struct RequestBody {
    sender: String,
    receiver: String,
    amount: u64,
    recent_blockhash: String,
}

#[derive(Serialize)]
pub struct ResponseBody {
    transaction: String,
}

pub async fn handler(
    body: RequestBody,
    context: Context,
) -> Result<impl Reply, Rejection> {
    let sender = Pubkey::from_str(&body.sender).map_err(|_| warp::reject())?;
    let receiver = Pubkey::from_str(&body.receiver).map_err(|_| warp::reject())?;
    let recent_blockhash = Hash::from_str(&body.recent_blockhash).map_err(|_| warp::reject())?;

    let transaction = context
        .relayer
        .create_transfer_transaction(sender, receiver, body.amount, recent_blockhash)
        .await
        .map_err(|_| warp::reject())?;

    let serialized = bincode::serialize(&transaction).map_err(|_| warp::reject())?;
    let base64_transaction = base64::encode(serialized);

    Ok(warp::reply::json(&ResponseBody {
        transaction: base64_transaction,
    }))
}￼Enter
