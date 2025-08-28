# Color codes for terminal output
RED := \033[0;31m
GREEN := \033[0;32m
YELLOW := \033[1;33m
BLUE := \033[0;34m
MAGENTA := \033[0;35m
CYAN := \033[0;36m
BOLD := \033[1m
RESET := \033[0m

# Common configuration
TEST_PORT := 8080
TEST_RPC_URL := http://127.0.0.1:8899
TEST_SIGNERS_CONFIG := tests/src/common/fixtures/signers.toml
TEST_SIGNERS_TURNKEY_CONFIG := tests/src/common/fixtures/signers-turnkey.toml
TEST_SIGNERS_PRIVY_CONFIG := tests/src/common/fixtures/signers-privy.toml
MULTI_SIGNERS_CONFIG := tests/src/common/fixtures/multi-signers.toml
REGULAR_CONFIG := tests/src/common/fixtures/kora-test.toml
AUTH_CONFIG := tests/src/common/fixtures/auth-test.toml
PAYMENT_ADDRESS_CONFIG := tests/src/common/fixtures/paymaster-address-test.toml

# CI-aware timeouts
VALIDATOR_TIMEOUT := $(if $(CI),20,30)
SERVER_TIMEOUT := $(if $(CI),20,30)

# Output control patterns
QUIET_OUTPUT := >/dev/null 2>&1
TEST_OUTPUT_FILTER := 2>&1 | grep -E "(test |running |ok$$|FAILED|failed|error:|Error:|ERROR)" | grep -v "running 0 tests" || true

# Clean structured output functions
define print_header
	@printf "\n$(BOLD)$(BLUE)================================================================================\n"
	@printf "  $(1)\n"
	@printf "================================================================================\n$(RESET)"
endef

define print_phase
	@printf "\n$(BOLD)$(CYAN)[Phase $(1)] $(2)$(RESET)\n"
	@printf "$(CYAN)--------------------------------------------------------------------------------$(RESET)\n"
endef

define print_step
	@printf "  $(GREEN)→$(RESET) $(1)\n"
endef

define print_substep
	@printf "    $(YELLOW)•$(RESET) $(1)\n"
endef

define print_success
	@printf "  $(GREEN)✓$(RESET) $(1)\n"
endef

define print_error
	@printf "  $(RED)✗$(RESET) $(1)\n"
endef

# Solana validator lifecycle management functions
define start_solana_validator
	$(call print_step,Starting Solana test validator...)
	@pkill -f "solana-test-validator" 2>/dev/null || true
	@sleep 2
	@rm -rf test-ledger 2>/dev/null || true
	@solana-test-validator --reset --quiet $(QUIET_OUTPUT) &
	@echo $$! > .validator.pid
	@counter=0; \
	while [ $$counter -lt $(VALIDATOR_TIMEOUT) ]; do \
		if solana cluster-version --url $(TEST_RPC_URL) >/dev/null 2>&1; then \
			break; \
		fi; \
		sleep 1; \
		counter=$$((counter + 1)); \
	done; \
	if [ $$counter -eq $(VALIDATOR_TIMEOUT) ]; then \
		printf "  $(RED)✗$(RESET) Validator failed to start\n"; \
		exit 1; \
	fi
	$(call print_substep,Validator ready on port 8899)
endef

define stop_solana_validator
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
# Usage: $(call start_kora_server,description,cargo_cmd,cargo_flags,config_file,setup_env,signers_config)
define start_kora_server
	@$(call stop_kora_server)
	@$(if $(5),\
		printf "    $(YELLOW)•$(RESET) Setting up test environment...\n"; \
		KORA_PRIVATE_KEY="$$(cat tests/src/common/local-keys/fee-payer-local.json)" cargo run -p tests --bin setup_test_env $(QUIET_OUTPUT) || exit 1;)
	$(call print_substep,Starting Kora server with $(1) configuration...)
	@$(if $(2),\
		KORA_PRIVATE_KEY="$$(cat tests/src/common/local-keys/fee-payer-local.json)" $(2) -p kora-cli --bin kora $(3) -- --config $(4) --rpc-url $(TEST_RPC_URL) rpc start --signers-config $(or $(6),$(TEST_SIGNERS_CONFIG)) --port $(TEST_PORT) $(QUIET_OUTPUT) &,\
		make run $(QUIET_OUTPUT) &)
	@echo $$! > .kora.pid
	@counter=0; \
	while [ $$counter -lt $(SERVER_TIMEOUT) ]; do \
		if curl -s http://127.0.0.1:$(TEST_PORT)/liveness >/dev/null 2>&1; then \
			break; \
		fi; \
		sleep 1; \
		counter=$$((counter + 1)); \
	done; \
	if [ $$counter -eq $(SERVER_TIMEOUT) ]; then \
		printf "  $(RED)✗$(RESET) Kora server failed to start\n"; \
		if [ -f .kora.pid ]; then \
			printf "    $(YELLOW)•$(RESET) PID: $$(cat .kora.pid)\n"; \
		fi; \
		exit 1; \
	fi
	$(call print_substep,Server ready on port $(TEST_PORT))
endef

# Server lifecycle management functions
define stop_kora_server
	@if [ -f .kora.pid ]; then \
		PID=$$(cat .kora.pid); \
		if kill -0 $$PID 2>/dev/null; then \
			kill -TERM $$PID 2>/dev/null || true; \
			sleep 2; \
			if kill -0 $$PID 2>/dev/null; then \
				kill -9 $$PID 2>/dev/null || true; \
			fi; \
		fi; \
		rm -f .kora.pid; \
	fi; \
	pkill -f "kora.*rpc.*start" 2>/dev/null || true; \
	sleep 1; \
	lsof -ti:$(TEST_PORT) | xargs kill -9 2>/dev/null || true; \
	sleep 1
endef

define run_integration_phase
	$(call print_phase,$(1),$(2))
	$(call print_step,Configuring test environment)
	@$(call start_kora_server,$(2),cargo run,,$(3),$(4),$(7))
	@$(if $(6),\
		printf "    $(YELLOW)•$(RESET) Initializing payment ATAs...\n"; \
		KORA_PRIVATE_KEY="$$(cat tests/src/common/local-keys/fee-payer-local.json)" cargo run -p kora-cli --bin kora -- --config $(3) --rpc-url $(TEST_RPC_URL) rpc initialize-atas --signers-config $(or $(7),$(TEST_SIGNERS_CONFIG)) $(QUIET_OUTPUT) || exit 1; \
		printf "    $(YELLOW)•$(RESET) Payment ATAs ready\n";)
	$(call print_step,Running tests...)
	@cargo test -p tests --quiet $(5) -- --nocapture $(QUIET_OUTPUT) || exit 1
	@printf "  $(GREEN)✓$(RESET) Tests passed\n"
	@$(call stop_kora_server)
	$(call print_success,Phase $(1) complete)
endef

define run_multi_signer_phase
	$(call print_phase,$(1),$(2))
	@$(call stop_kora_server)
	@if [ ! -f "tests/src/common/local-keys/fee-payer-local.json" ]; then \
		$(call print_error,fee-payer-local.json not found); \
		exit 1; \
	fi
	@if [ ! -f "tests/src/common/local-keys/signer2-local.json" ]; then \
		$(call print_error,signer2-local.json not found); \
		printf "    Create it with: solana-keygen new --outfile tests/src/common/local-keys/signer2-local.json\n"; \
		exit 1; \
	fi
	$(call print_step,Configuring multi-signer environment)
	$(call print_substep,Setting up test accounts...)
	@KORA_PRIVATE_KEY="$$(cat tests/src/common/local-keys/fee-payer-local.json)" \
	 KORA_PRIVATE_KEY_2="$$(cat tests/src/common/local-keys/signer2-local.json)" \
	 cargo run -p tests --bin setup_test_env $(QUIET_OUTPUT) || exit 1
	$(call print_substep,Starting server with multi-signer configuration...)
	@KORA_PRIVATE_KEY="$$(cat tests/src/common/local-keys/fee-payer-local.json)" \
	 KORA_PRIVATE_KEY_2="$$(cat tests/src/common/local-keys/signer2-local.json)" \
	 cargo run -p kora-cli --bin kora -- --config $(3) --rpc-url $(TEST_RPC_URL) rpc start --signers-config $(4) --port $(TEST_PORT) $(QUIET_OUTPUT) &
	@echo $$! > .kora.pid
	@sleep 5
	$(call print_substep,Server ready on port $(TEST_PORT))
	$(call print_step,Running tests...)
	@cargo test -p tests --quiet $(5) -- --nocapture $(QUIET_OUTPUT) || exit 1
	@printf "  $(GREEN)✓$(RESET) Tests passed\n"
	@$(call stop_kora_server)
	$(call print_success,Phase $(1) complete)
endef

# Setup test environment
setup-test-env:
	$(call print_step,Setting up test environment...)
	@KORA_PRIVATE_KEY="$$(cat tests/src/common/local-keys/fee-payer-local.json)" \
	cargo run -p tests --bin setup_test_env $(QUIET_OUTPUT)
	$(call print_success,Test environment ready)

# Clean up any running validators
clean-validator:
	@$(call stop_solana_validator)

# Clean up any running Kora nodes
clean-kora:
	@$(call stop_kora_server)

# Clean up both validator and Kora node
clean-test-env: clean-validator clean-kora
	$(call print_success,Test environment cleaned up)

# Generate a random key that can be used as an API key or as an HMAC secret
generate-key:
	@openssl rand -hex 32