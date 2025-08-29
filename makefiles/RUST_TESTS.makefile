# Run unit tests only (no integration tests)
test:
	@cargo test --lib --workspace --exclude tests --quiet 2>/dev/null || true

# Run all integration tests with clean output
test-integration:
	$(call print_header,KORA INTEGRATION TEST SUITE)
	$(call print_step,Initializing test infrastructure)
	@$(call start_solana_validator)
	$(call print_substep,Setting up base test environment...)
	@KORA_PRIVATE_KEY="$$(cat tests/src/common/local-keys/fee-payer-local.json)" cargo run -p tests --bin setup_test_env $(QUIET_OUTPUT)
	$(call print_success,Infrastructure ready)
	
	@$(call run_integration_phase,1,RPC tests,$(REGULAR_CONFIG),,--test rpc,)
	@$(call run_integration_phase,2,token tests,$(REGULAR_CONFIG),,--test tokens,)
	@$(call run_integration_phase,3,external tests,$(REGULAR_CONFIG),,--test external,)
	@$(call run_integration_phase,4,auth tests,$(AUTH_CONFIG),,--test auth,)
	@$(call run_integration_phase,5,payment address tests,$(PAYMENT_ADDRESS_CONFIG),,--test payment_address,true)
	@$(call run_multi_signer_phase,6,multi-signer tests,$(REGULAR_CONFIG),$(MULTI_SIGNERS_CONFIG),--test multi_signer)

	$(call print_header,TEST SUITE COMPLETE)
	@$(call stop_solana_validator)

# Individual test targets for development
test-regular:
	$(call print_header,REGULAR TESTS)
	@$(call start_solana_validator)
	@cargo run -p tests --bin setup_test_env $(QUIET_OUTPUT)
	$(call run_integration_phase,1,Regular Tests,$(REGULAR_CONFIG),,--test rpc,)
	@$(call stop_solana_validator)

test-token:
	$(call print_header,TOKEN TESTS)
	@$(call start_solana_validator)
	@cargo run -p tests --bin setup_test_env $(QUIET_OUTPUT)
	$(call run_integration_phase,1,Tokens Tests,$(REGULAR_CONFIG),,--test tokens,)
	@$(call stop_solana_validator)

test-auth:
	$(call print_header,AUTHENTICATION TESTS)
	@$(call start_solana_validator)
	@cargo run -p tests --bin setup_test_env $(QUIET_OUTPUT)
	$(call run_integration_phase,1,Authentication Tests,$(AUTH_CONFIG),,--test auth,)
	@$(call stop_solana_validator)

test-payment:
	$(call print_header,PAYMENT ADDRESS TESTS)
	@$(call start_solana_validator)
	@cargo run -p tests --bin setup_test_env $(QUIET_OUTPUT)
	$(call run_integration_phase,1,Payment Address Tests,$(PAYMENT_ADDRESS_CONFIG),,--test payment_address,true)
	@$(call stop_solana_validator)

test-multi-signer:
	$(call print_header,MULTI-SIGNER TESTS)
	@$(call start_solana_validator)
	$(call run_multi_signer_phase,1,Multi-Signer Tests,$(REGULAR_CONFIG),$(MULTI_SIGNERS_CONFIG),--test multi-signers)
	@$(call stop_solana_validator)