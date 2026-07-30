#![no_main]

use kora_lib::transaction::TransactionUtil;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let encoded = String::from_utf8_lossy(data);
    let _ = TransactionUtil::decode_b64_transaction(&encoded);
});
