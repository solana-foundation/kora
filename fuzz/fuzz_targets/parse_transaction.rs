#![no_main]

use kora_lib::transaction::VersionedTransactionResolved;
use libfuzzer_sys::fuzz_target;
use solana_sdk::transaction::VersionedTransaction;

fuzz_target!(|data: &[u8]| {
    let Ok(transaction) = bincode::deserialize::<VersionedTransaction>(data) else {
        return;
    };

    let Ok(mut resolved) = VersionedTransactionResolved::from_kora_built_transaction(&transaction)
    else {
        return;
    };

    let _ = resolved.get_or_parse_system_instructions();
    let _ = resolved.get_or_parse_spl_instructions();
    let _ = resolved.get_or_parse_alt_instructions();
    let _ = resolved.get_or_parse_loader_v4_instructions();
    let _ = resolved.get_or_parse_bpf_loader_upgradeable_instructions();
    let _ = resolved.get_or_parse_token2022_security_instructions();
});
