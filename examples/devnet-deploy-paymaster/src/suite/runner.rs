use std::path::Path;

use anyhow::Result;

use super::{adversarial, happy, harness::Harness};

pub async fn run_full(
    kora_url: &str,
    rpc_url: &str,
    program_so: &Path,
    happy_phase: bool,
    adversarial_phase: bool,
) -> Result<bool> {
    let mut ok = true;

    if happy_phase {
        println!("== happy path ==");
        happy::run(kora_url, rpc_url, program_so).await?;
        println!("  OK");
        println!();
    }

    if adversarial_phase {
        println!("== adversarial ==");
        let h = Harness::connect(kora_url, rpc_url).await?;
        println!("  fee payer: {} ({:.4} SOL)", h.payer(), h.balance().await? as f64 / 1e9);
        let bytes = std::fs::read(program_so)?;
        let report = adversarial::run(&h, &bytes).await?;
        println!("  signed (findings): {}/{}", report.signed, report.total);
        ok &= report.signed == 0;
    }

    Ok(ok)
}
