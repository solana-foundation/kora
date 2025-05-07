#[post("/action/transferTransaction", data = "<payload>")]
pub async fn transfer_transaction_route(
    payload: Json<TransferRequest>,
    state: &State<AppState>,
) -> Result<Json<TransferResponse>, status::BadRequest<String>> {
    use crate::lib::transactions::create_transfer_transaction;

    let rpc = &state.rpc_client;
    let keypair = state.load_keypair()?; // Load from config or signer

    let to = Pubkey::from_str(&payload.to).map_err(|e| status::BadRequest(Some(e.to_string())))?;
    let lamports = payload.lamports;

    let tx = create_transfer_transaction(rpc, &keypair, &to, lamports, None)
        .map_err(|e| status::BadRequest(Some(e.to_string())))?;

    let tx_bytes = bincode::serialize(&tx).unwrap();

    Ok(Json(TransferResponse {
        signed_tx: base64::encode(tx_bytes),
    }))
}
