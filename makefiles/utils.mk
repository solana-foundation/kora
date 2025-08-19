# Common configuration
TEST_PORT := 8080
TEST_RPC_URL := http://127.0.0.1:8899
TEST_SIGNERS_CONFIG := tests/src/common/fixtures/signers.toml
TEST_SIGNERS_TURNKEY_CONFIG := tests/src/common/fixtures/signers-turnkey.toml
TEST_SIGNERS_PRIVY_CONFIG := tests/src/common/fixtures/signers-privy.toml
REGULAR_CONFIG := tests/src/common/fixtures/kora-test.toml
AUTH_CONFIG := tests/src/common/fixtures/auth-test.toml
PAYMENT_ADDRESS_CONFIG := tests/src/common/fixtures/paymaster-address-test.toml

# Output control patterns
QUIET_OUTPUT := >/dev/null 2>&1
TEST_OUTPUT_FILTER := 2>&1 | grep -E "(test |running |ok$$|FAILED|failed|error:|Error:|ERROR)" || true


# Solana validator lifecycle management functions
define start_solana_validator
	@echo "🚀 Starting local Solana test validator..."
	@solana-test-validator --reset --quiet &
	@echo $$! > .validator.pid
	@echo "⏳ Waiting for validator to start..."
	@sleep 5
endef

define stop_solana_validator
	@printf "Stopping Solana validator...\n"
	@if [ -f .validator.pid ]; then \
		PID=$$(cat .validator.pid); \
		if kill -0 $$PID 2>/dev/null; then \
			kill $$PID 2>/dev/null || true; \
			sleep 1; \
			kill -9 $$PID 2>/dev/null || true; \
		fi; \
		rm -f .validator.pid; \
	fi; \
	pkill -f "solana-test-validator" 2>/dev/null || true; \
	sleep 2; \
	rm -rf test-ledger 2>/dev/null || true
endef

# Start Kora server with flexible configuration
# Usage: $(call start_kora_server,description,cargo_cmd,cargo_flags,config_file,setup_env)
define start_kora_server
	@$(call stop_kora_server)
	@$(if $(5),\
		printf "🔧 Setting up test environment...\n"; \
		KORA_PRIVATE_KEY="$$(cat tests/src/common/local-keys/fee-payer-local.json)" cargo run -p tests --bin setup_test_env;)
	@echo "🚀 Starting Kora $(1)..."
	@$(if $(2),\
		KORA_PRIVATE_KEY="$$(cat tests/src/common/local-keys/fee-payer-local.json)" $(2) -p kora-cli --bin kora $(3) -- --config $(4) --rpc-url $(TEST_RPC_URL) rpc start --signers-config $(TEST_SIGNERS_CONFIG) --port $(TEST_PORT) $(QUIET_OUTPUT) &,\
		make run &)
	@echo $$! > .kora.pid
	@echo "⏳ Waiting for server to start..."
	@sleep 5
endef

# Server lifecycle management functions
define stop_kora_server
	@printf "Stopping Kora server...\n"
	@if [ -f .kora.pid ]; then \
		PID=$$(cat .kora.pid); \
		if kill -0 $$PID 2>/dev/null; then \
			kill $$PID 2>/dev/null || true; \
			sleep 1; \
			kill -9 $$PID 2>/dev/null || true; \
		fi; \
		rm -f .kora.pid; \
	fi; \
	pkill -f "kora" 2>/dev/null || true; \
	sleep 2
endef


define run_integration_phase
	@echo "📋 Phase $(1): $(2)"
	@$(call start_kora_server,$(2),cargo run,,$(3),$(4))
	@$(if $(6),\
		echo "🔧 Initializing payment ATAs..."; \
		KORA_PRIVATE_KEY="$$(cat tests/src/common/local-keys/fee-payer-local.json)" cargo run -p kora-cli --bin kora -- --config $(3) --rpc-url $(TEST_RPC_URL) rpc initialize-atas --signers-config $(TEST_SIGNERS_CONFIG);)
	@echo "🧪 Running $(1) integration tests..."
	@cargo test -p tests --quiet $(5) -- --nocapture $(TEST_OUTPUT_FILTER)
	@$(call stop_kora_server)
endef

# Setup test environment
setup-test-env:
	KORA_PRIVATE_KEY="$$(cat tests/src/common/local-keys/fee-payer-local.json)" cargo run -p tests --bin setup_test_env

# Clean up any running validators
clean-validator:
	@$(call stop_solana_validator)

# Clean up any running Kora nodes
clean-kora:
	@$(call stop_kora_server)

# Clean up both validator and Kora node
clean-test-env: clean-validator clean-kora
	@echo "✅ Test environment cleaned up"

# Generate a random key that can be used as an API key or as an HMAC secret
generate-key:
	openssl rand -hex 32