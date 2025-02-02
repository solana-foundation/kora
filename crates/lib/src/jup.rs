use jup_ag::{Quote, QuoteConfig};

use crate::constant::NATIVE_SOL;

pub async fn get_quote(starting_mint: String, amount_to_swap: u64) -> Result<Quote, jup_ag::Error> {
    let sol = NATIVE_SOL.parse().unwrap();
    let input = starting_mint.parse().unwrap();

    let quote = jup_ag::quote(
        input,
        sol,
        amount_to_swap,
        QuoteConfig {
            only_direct_routes: false,
            as_legacy_transaction: None,
            slippage_bps: Some(10),
            ..Default::default()
        },
    )
    .await?;

    Ok(quote)
}
