// Adversarial Restrictive Tests
//
// CONFIG: Uses tests/src/common/fixtures/fee-payer-policy-test.toml (restrictive policies)
// TESTS: Fee payer policy violation testing with all policies disabled
//        - SOL transfer policy violations (allow_sol_transfers = false)
//        - SPL transfer policy violations (allow_spl_transfers = false)
//        - Token2022 transfer policy violations (allow_token2022_transfers = false)
//        - Assignment policy violations (allow_assign = false)
//        - Burn policy violations (allow_burn = false)
//        - Close account policy violations (allow_close_account = false)
//        - Approve policy violations (allow_approve = false)

mod fee_payer_policy_violations;

// Make common utilities available
#[path = "../src/common/mod.rs"]
mod common;
