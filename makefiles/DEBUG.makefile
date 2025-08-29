# DEBUG.makefile - Verbose test execution with debug logging
# This makefile provides the same test functionality as the main makefiles
# but with verbose output and RUST_LOG=debug for debugging purposes
# 
# Usage: make -f makefiles/DEBUG.makefile debug-test-integration

# Include utils but override quiet settings
include makefiles/UTILS.makefile

# Override quiet settings for verbose output
override QUIET_OUTPUT := 
override TEST_OUTPUT_FILTER := 

# Export RUST_LOG=debug for all subprocesses
export RUST_LOG := debug

# Clean up any existing processes before starting debug tests
define debug_cleanup
	@killall kora 2>/dev/null || true
	@killall solana-test-validator 2>/dev/null || true
endef

debug-test-regular:
	$(call print_header,DEBUG REGULAR TESTS)
	$(debug_cleanup)
	@$(call start_solana_validator)
	@cargo run -p tests --bin setup_test_env
	@$(call run_integration_phase,1,Regular Tests,$(REGULAR_CONFIG),,--test rpc,)
	@$(call stop_solana_validator)

debug-test-token:
	$(call print_header,DEBUG TOKEN TESTS)
	$(debug_cleanup)
	@$(call start_solana_validator)
	@cargo run -p tests --bin setup_test_env
	@$(call run_integration_phase,1,Token Tests,$(REGULAR_CONFIG),,--test tokens,)
	@$(call stop_solana_validator)

debug-test-auth:
	$(call print_header,DEBUG AUTHENTICATION TESTS)
	$(debug_cleanup)
	@$(call start_solana_validator)
	@cargo run -p tests --bin setup_test_env
	@$(call run_integration_phase,1,Authentication Tests,$(AUTH_CONFIG),,--test auth,)
	@$(call stop_solana_validator)

debug-test-payment:
	$(call print_header,DEBUG PAYMENT ADDRESS TESTS)
	$(debug_cleanup)
	@$(call start_solana_validator)
	@cargo run -p tests --bin setup_test_env
	@$(call run_integration_phase,1,Payment Address Tests,$(PAYMENT_ADDRESS_CONFIG),,--test payment_address,true)
	@$(call stop_solana_validator)

debug-test-multi-signer:
	$(call print_header,DEBUG Multi-Signer Test Suite)
	$(debug_cleanup)
	@$(call start_solana_validator)
	@$(call run_multi_signer_phase,1,Multi-Signer Tests,$(REGULAR_CONFIG),$(MULTI_SIGNERS_CONFIG),--test multi-signer)
	@$(call stop_solana_validator)

debug-test-integration:
	$(call print_header,DEBUG KORA INTEGRATION TEST SUITE)
	$(debug_cleanup)
	$(call print_step,Initializing test infrastructure with debug logging)
	@$(call start_solana_validator)
	$(call print_substep,Setting up base test environment (DEBUG)...)
	@KORA_PRIVATE_KEY="$$(cat tests/src/common/local-keys/fee-payer-local.json)" cargo run -p tests --bin setup_test_env
	$(call print_success,Infrastructure ready)
	
	@$(call run_integration_phase,1,RPC tests,$(REGULAR_CONFIG),,--test rpc,)
	@$(call run_integration_phase,2,token tests,$(REGULAR_CONFIG),,--test tokens,)
	@$(call run_integration_phase,3,external tests,$(REGULAR_CONFIG),,--test external,)
	@$(call run_integration_phase,4,auth tests,$(AUTH_CONFIG),,--test auth,)
	@$(call run_integration_phase,5,payment address tests,$(PAYMENT_ADDRESS_CONFIG),,--test payment_address,true)
	@$(call run_multi_signer_phase,6,multi-signer tests,$(REGULAR_CONFIG),$(MULTI_SIGNERS_CONFIG),--test multi_signer)

	$(call print_header,DEBUG TEST SUITE COMPLETE)
	@$(call stop_solana_validator)

debug-setup-test-env:
	$(call print_step,Setting up test environment with debug logging...)
	@KORA_PRIVATE_KEY="$$(cat tests/src/common/local-keys/fee-payer-local.json)" \
	cargo run -p tests --bin setup_test_env
	$(call print_success,Test environment ready (DEBUG))

debug-test-all: debug-test-integration

.PHONY: debug-test-integration debug-test-regular debug-test-token debug-test-auth debug-test-payment debug-test-multi-signer debug-test-all debug-setup-test-env