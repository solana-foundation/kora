// Gas Swap Plugin Integration Tests
//
// CONFIG: Uses tests/src/common/fixtures/gas-swap-plugin-test.toml (GasSwap plugin enabled)
// TESTS: Verifies the GasSwap plugin enforces exactly 2 outer instructions:
//        ix[0] = SplTokenTransfer, ix[1] = SystemTransfer

mod gas_swap_plugin_tests;

#[path = "../src/common/mod.rs"]
mod common;
