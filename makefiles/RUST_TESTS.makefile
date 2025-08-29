# Run unit tests only (no integration tests)
test:
	@cargo test --lib --workspace --exclude tests --quiet 2>/dev/null || true

# Run all integration tests with clean output
test-integration:
	$(call print_header,KORA INTEGRATION TEST SUITE)
	$(call print_step,Initializing shared test infrastructure)
	@$(call start_solana_validator)
	$(call print_substep,Setting up base test environment...)
	@cargo run -p tests --bin setup_test_env $(QUIET_OUTPUT)
	@if [ ! -f "tests/src/common/local-keys/signer2-local.json" ]; then \
		$(call print_substep,Creating signer2-local.json for multi-signer tests...); \
		solana-keygen new --outfile tests/src/common/local-keys/signer2-local.json --no-bip39-passphrase --silent; \
	fi
	$(call print_substep,Setting up multi-signer test environment...)
	@KORA_PRIVATE_KEY="$$(cat tests/src/common/local-keys/fee-payer-local.json)" \
	 KORA_PRIVATE_KEY_2="$$(cat tests/src/common/local-keys/signer2-local.json)" \
	 cargo run -p tests --bin setup_test_env $(QUIET_OUTPUT)
	$(call print_success,Infrastructure ready)
	
	$(call print_step,Starting all test servers in parallel...)
	@$(call start_kora_parallel,RPC,$(REGULAR_CONFIG),$(RPC_PORT),,$(TEST_SIGNERS_CONFIG))
	@$(call start_kora_parallel,Tokens,$(REGULAR_CONFIG),$(TOKENS_PORT),,$(TEST_SIGNERS_CONFIG))
	@$(call start_kora_parallel,External,$(REGULAR_CONFIG),$(EXTERNAL_PORT),,$(TEST_SIGNERS_CONFIG))
	@$(call start_kora_parallel,Auth,$(AUTH_CONFIG),$(AUTH_PORT),,$(TEST_SIGNERS_CONFIG))
	@$(call start_kora_parallel,Payment,$(PAYMENT_ADDRESS_CONFIG),$(PAYMENT_PORT),true,$(TEST_SIGNERS_CONFIG))
	@$(call start_kora_parallel,Multi-Signer,$(REGULAR_CONFIG),$(MULTI_SIGNER_PORT),,$(MULTI_SIGNERS_CONFIG))
	@sleep 8
	$(call print_success,All servers ready)
	
	$(call print_step,Running all test suites in parallel...)
	$(call print_substep,RPC Tests running...)
	$(call print_substep,Tokens Tests running...)
	$(call print_substep,External Tests running...)
	$(call print_substep,Auth Tests running...)
	$(call print_substep,Payment Tests running...)
	$(call print_substep,Multi-Signer Tests running...)
	@START_TIME=$$(date +%s); \
	 TEST_SERVER_URL=http://127.0.0.1:$(RPC_PORT) cargo test -p tests --test rpc --quiet >/dev/null 2>&1 & \
	 RPC_PID=$$!; \
	 TEST_SERVER_URL=http://127.0.0.1:$(TOKENS_PORT) cargo test -p tests --test tokens --quiet >/dev/null 2>&1 & \
	 TOKENS_PID=$$!; \
	 TEST_SERVER_URL=http://127.0.0.1:$(EXTERNAL_PORT) cargo test -p tests --test external --quiet >/dev/null 2>&1 & \
	 EXTERNAL_PID=$$!; \
	 TEST_SERVER_URL=http://127.0.0.1:$(AUTH_PORT) cargo test -p tests --test auth --quiet >/dev/null 2>&1 & \
	 AUTH_PID=$$!; \
	 TEST_SERVER_URL=http://127.0.0.1:$(PAYMENT_PORT) cargo test -p tests --test payment_address --quiet >/dev/null 2>&1 & \
	 PAYMENT_PID=$$!; \
	 TEST_SERVER_URL=http://127.0.0.1:$(MULTI_SIGNER_PORT) cargo test -p tests --test multi_signer --quiet >/dev/null 2>&1 & \
	 MULTI_PID=$$!; \
	 wait $$RPC_PID && printf "  $(GREEN)✓$(RESET) RPC Tests: $(GREEN)PASSED$(RESET)\n" || printf "  $(RED)✗$(RESET) RPC Tests: $(RED)FAILED$(RESET)\n"; \
	 wait $$TOKENS_PID && printf "  $(GREEN)✓$(RESET) Tokens Tests: $(GREEN)PASSED$(RESET)\n" || printf "  $(RED)✗$(RESET) Tokens Tests: $(RED)FAILED$(RESET)\n"; \
	 wait $$EXTERNAL_PID && printf "  $(GREEN)✓$(RESET) External Tests: $(GREEN)PASSED$(RESET)\n" || printf "  $(RED)✗$(RESET) External Tests: $(RED)FAILED$(RESET)\n"; \
	 wait $$AUTH_PID && printf "  $(GREEN)✓$(RESET) Auth Tests: $(GREEN)PASSED$(RESET)\n" || printf "  $(RED)✗$(RESET) Auth Tests: $(RED)FAILED$(RESET)\n"; \
	 wait $$PAYMENT_PID && printf "  $(GREEN)✓$(RESET) Payment Tests: $(GREEN)PASSED$(RESET)\n" || printf "  $(RED)✗$(RESET) Payment Tests: $(RED)FAILED$(RESET)\n"; \
	 wait $$MULTI_PID && printf "  $(GREEN)✓$(RESET) Multi-Signer Tests: $(GREEN)PASSED$(RESET)\n" || printf "  $(RED)✗$(RESET) Multi-Signer Tests: $(RED)FAILED$(RESET)\n"; \
	 END_TIME=$$(date +%s); \
	 DURATION=$$((END_TIME - START_TIME)); \
	 printf "  $(YELLOW)•$(RESET) All tests completed in $${DURATION}s\n"
	
	$(call print_step,Cleaning up...)
	@$(call stop_all_parallel_servers)
	@$(call stop_solana_validator)
	$(call print_header,PARALLEL TESTS COMPLETE)

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
