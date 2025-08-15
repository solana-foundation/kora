# Run TypeScript SDK tests with local validator and Kora node
test-ts-unit:
	@printf "Running TypeScript SDK tests (unit tests)...\n"
	-@cd sdks/ts && pnpm test:unit


test-ts-integration-basic:
	@$(call start_solana_validator)
	@$(call start_kora_server,node for TS tests,cargo run,,$(REGULAR_CONFIG),)
	@printf "Running TypeScript SDK tests (basic config)...\n"
	-@cd sdks/ts && pnpm test:integration
	@$(call stop_kora_server)
	@$(call stop_solana_validator)

# Run TypeScript auth tests specifically (using integration tests with auth enabled)
test-ts-integration-auth:
	@$(call start_solana_validator)
	@$(call start_kora_server,node for TS auth tests,cargo run,,$(AUTH_CONFIG),)
	@printf "Running TypeScript SDK auth tests...\n"
	-@cd sdks/ts && pnpm test:integration:auth
	@$(call stop_kora_server)
	@$(call stop_solana_validator)

# Run TypeScript tests with Turnkey signer
test-ts-integration-turnkey:
	@$(call start_solana_validator)
	@echo "🚀 Starting Kora node with Turnkey signer..."
	@$(call stop_kora_server)
	@cargo run -p kora-cli --bin kora -- --config $(REGULAR_CONFIG) --rpc-url $(TEST_RPC_URL) rpc --with-turnkey-signer --port $(TEST_PORT) $(QUIET_OUTPUT) &
	@echo $$! > .kora.pid
	@echo "⏳ Waiting for server to start..."
	@sleep 5
	@printf "Running TypeScript SDK tests with Turnkey signer...\n"
	-@cd sdks/ts && pnpm test:integration:turnkey
	@$(call stop_kora_server)
	@$(call stop_solana_validator)

# Run TypeScript tests with Privy signer  
test-ts-integration-privy:
	@$(call start_solana_validator)
	@echo "🚀 Starting Kora node with Privy signer..."
	@$(call stop_kora_server)
	@cargo run -p kora-cli --bin kora -- --config $(REGULAR_CONFIG) --rpc-url $(TEST_RPC_URL) rpc --with-privy-signer --port $(TEST_PORT) $(QUIET_OUTPUT) &
	@echo $$! > .kora.pid
	@echo "⏳ Waiting for server to start..."
	@sleep 5
	@printf "Running TypeScript SDK tests with Privy signer...\n"
	-@cd sdks/ts && pnpm test:integration:privy
	@$(call stop_kora_server)
	@$(call stop_solana_validator)

# Run all signer tests
test-ts-signers: test-ts-integration-turnkey test-ts-integration-privy

# Run all TypeScript SDK tests (no signers b/c api rate limits)
test-ts: test-ts-unit test-ts-integration-basic test-ts-integration-auth # test-ts-signers
