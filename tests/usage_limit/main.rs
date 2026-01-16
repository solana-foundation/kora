// Usage Limit Integration Tests
//
// CONFIG: Uses tests/src/common/fixtures/usage-limit-test.toml (usage limits enabled)
// TESTS: Usage limit enforcement for transactions and instructions
//        - Transaction-level limits (lifetime and time-windowed)
//        - Instruction-level limits (System CreateAccount)
//        - Multiple rules enforcement
//        - Bundle-level limits (transaction and instruction)

mod bundle_limits;
mod instruction_limits;
mod multiple_rules;
mod transaction_limits;

// Make common utilities available
#[path = "../src/common/mod.rs"]
mod common;
