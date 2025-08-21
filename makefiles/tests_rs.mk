# Run tests
test:
	cargo test --lib --quiet



# Run all integration tests (regular + auth + payment address + multi-signer)
test-integration:
	@$(call start_solana_validator)
	@echo "🧪 Running all integration tests..."
	@echo "🔧 Setting up test environment..."
	@cargo run -p tests --bin setup_test_env $(QUIET_OUTPUT)
	$(call run_integration_phase,1,regular tests,$(REGULAR_CONFIG),,--test integration,)
	$(call run_integration_phase,2,auth tests,$(AUTH_CONFIG),,--test auth,)
	$(call run_integration_phase,3,payment address tests,$(PAYMENT_ADDRESS_CONFIG),,--test payment-address,true)
	$(call run_multi_signer_phase,4,multi-signer tests,$(REGULAR_CONFIG),$(MULTI_SIGNERS_CONFIG),--test multi-signers)
	@echo "✅ All integration tests completed"
	@$(call stop_solana_validator)
