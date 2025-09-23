// Adversarial Basic Tests
//
// CONFIG: Uses tests/src/common/fixtures/kora-test.toml (permissive policies)
// TESTS: Security and robustness testing with normal configuration
//        - Program validation attacks (disallowed programs)
//        - Invalid token states (frozen)
//        - Fee payer exploitation

mod fee_payer_exploitation;
mod program_validation;
mod token_states;

// Make common utilities available
#[path = "../src/common/mod.rs"]
mod common;
