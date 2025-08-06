.PHONY: check lint test build run clean all install generate-key setup-test-env test-integration test-integration-coverage test-all test-ts coverage coverage-clean build-bin build-lib build-rpc build-tk run-presigned openapi gen-ts-client

# Common configuration
TEST_PORT := 8080
TEST_PRIVATE_KEY := ./tests/testing-utils/local-keys/fee-payer-local.json
TEST_RPC_URL := http://127.0.0.1:8899
REGULAR_CONFIG := tests/kora-test.toml
AUTH_CONFIG := tests/fixtures/auth-test.toml

# Output control patterns
QUIET_OUTPUT := >/dev/null 2>&1
TEST_OUTPUT_FILTER := 2>&1 | grep -E "(test |running |ok$$|FAILED|failed|error:|Error:|ERROR)" || true
SETUP_OUTPUT := >/dev/null 2>&1


# Default target
all: check test build

# install
install:
	cargo install --path crates/cli
	cargo install --path crates/rpc

# Check code formatting
check:
	cargo fmt --all -- --check

# Run all fixes and checks
lint:
	cargo clippy --fix --allow-dirty -- -D warnings
	cargo fmt --all
	cargo fmt --all -- --check
	
# Run tests
test:
	cargo test --lib --quiet

# Generate a random key that can be used as an API key or as an HMAC secret
generate-key:
	openssl rand -hex 32

# Server lifecycle management functions
define stop_kora_server
	echo "🛑 Stopping Kora server..."
	if [ -f .kora.pid ]; then \
		PID=$$(cat .kora.pid); \
		if kill -0 $$PID 2>/dev/null; then \
			kill $$PID 2>/dev/null || true; \
			sleep 1; \
			kill -9 $$PID 2>/dev/null || true; \
		fi; \
		rm -f .kora.pid; \
	fi; \
	pkill -f "kora-rpc" 2>/dev/null || true; \
	sleep 2
endef

# Solana validator lifecycle management functions
define start_solana_validator
	echo "🚀 Starting local Solana test validator..."
	solana-test-validator --reset --quiet >/dev/null 2>&1 &
	echo $$! > .validator.pid
	echo "⏳ Waiting for validator to start..."
	sleep 5
endef

define stop_solana_validator
	echo "🛑 Stopping Solana validator..."
	if [ -f .validator.pid ]; then \
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
	$(call stop_kora_server)
	$(if $(5),\
		echo "🔧 Setting up test environment..."; \
		cargo run -p tests --bin setup-test-env $(SETUP_OUTPUT);)
	echo "🚀 Starting Kora $(1)..."
	$(if $(2),\
		$(2) -p kora-rpc --bin kora-rpc $(3) -- --private-key $(TEST_PRIVATE_KEY) --config $(4) --rpc-url $(TEST_RPC_URL) --port $(TEST_PORT) $(QUIET_OUTPUT) &,\
		make run >/dev/null 2>&1 &)
	echo $$! > .kora.pid
	echo "⏳ Waiting for server to start..."
	sleep 5
endef

define run_regular_tests
	@echo "🧪 Running regular integration tests..."
	@$(1) -p tests --quiet --test integration $(2) -- --skip auth_integration_tests $(TEST_OUTPUT_FILTER)
endef

define run_auth_tests
	@echo "🧪 Running auth integration tests..."
	@$(1) -p tests --quiet --test integration auth_integration_tests $(2) -- --nocapture $(TEST_OUTPUT_FILTER)
endef

define run_integration_phase
	@echo "📋 Phase $(1): $(2)"
	@$(call start_kora_server,$(2),$(3),$(4),$(5),)
	$(6)
	@$(call stop_kora_server)
endef

# Setup test environment
setup-test-env:
	cargo run -p tests --bin setup-test-env

# Run all integration tests (regular + auth)
test-integration:
	@$(call start_solana_validator)
	@echo "🧪 Running all integration tests..."
	@echo "🔧 Setting up test environment..."
	@cargo run -p tests --bin setup-test-env $(SETUP_OUTPUT)
	$(call run_integration_phase,1,regular tests,cargo run,,$(REGULAR_CONFIG),$(call run_regular_tests,cargo test,))
	$(call run_integration_phase,2,auth tests,cargo run,,$(AUTH_CONFIG),$(call run_auth_tests,cargo test,))
	@echo "✅ All integration tests completed"
	@$(call stop_solana_validator)

# Run all integration tests with coverage instrumentation (for CI)
test-integration-coverage:
	@echo "🧪 Running all integration tests with coverage..."
	@echo "🔧 Setting up test environment..."
	@cargo run -p tests --bin setup-test-env $(SETUP_OUTPUT)
	$(call run_integration_phase,1,regular tests,cargo llvm-cov run,--no-report,$(REGULAR_CONFIG),$(call run_regular_tests,cargo llvm-cov test,--no-report))
	$(call run_integration_phase,2,auth tests,cargo llvm-cov run,--no-report,$(AUTH_CONFIG),$(call run_auth_tests,cargo llvm-cov test,--no-report))
	@echo "✅ All integration tests completed"


# Run TypeScript SDK tests with local validator and Kora node
test-ts:
	@$(call start_solana_validator)
	@$(call start_kora_server,node for TS tests,,,,)
	@echo "🧪 Running TypeScript SDK tests..."
	@cd sdks/ts && pnpm test; \
	TEST_EXIT_CODE=$$?; \
	$(call stop_kora_server); \
	$(call stop_solana_validator); \
	exit $$TEST_EXIT_CODE

test-all: test test-integration # test-ts

# Clean up any running validators
clean-validator:
	@$(call stop_solana_validator)

# Clean up any running Kora nodes
clean-kora:
	@$(call stop_kora_server)

# Clean up both validator and Kora node
clean-test-env: clean-validator clean-kora
	@echo "✅ Test environment cleaned up"

# Build all binaries
build:
	cargo build --workspace

# Build specific binary
build-bin:
	cargo build --bin $(bin)

# Build lib
build-lib:
	cargo build -p kora-lib

# Build rpc
build-rpc:
	cargo build -p kora-rpc

# Build tk-rs
build-tk:
	cargo build -p tk-rs

# Run presigned release binary
run-presigned:
	cargo run --bin presigned

# Run with default configuration
run:
	cargo run -p kora-rpc --bin kora-rpc -- --private-key ./tests/testing-utils/local-keys/fee-payer-local.json --config tests/kora-test.toml --rpc-url http://127.0.0.1:8899


# Clean build artifacts
clean:
	cargo clean

# Gen openapi docs
openapi:
	cargo run -p kora-rpc --bin kora-openapi --features docs

# Generate TypeScript client
gen-ts-client:
	docker run --rm -v "${PWD}:/local" openapitools/openapi-generator-cli generate \
		-i /local/crates/rpc/src/openapi/spec/combined_api.json \
		-g typescript-fetch \
		-o /local/generated/typescript-client \
		--additional-properties=supportsES6=true,npmName=kora-client,npmVersion=0.1.0

# Helper function to check and install cargo-llvm-cov and llvm-tools-preview
define check_coverage_tool
        @if ! command -v cargo-llvm-cov >/dev/null 2>&1; then \
                echo "🔧 cargo-llvm-cov not found, installing..."; \
                cargo install cargo-llvm-cov; \
        fi
        @if ! rustup component list --installed | grep -q llvm-tools-preview; then \
                echo "🔧 Installing llvm-tools-preview..."; \
                rustup component add llvm-tools-preview; \
        fi
  endef

# Generate HTML coverage report (unit tests only)
coverage:
	$(call check_coverage_tool)
	@echo "🧪 Generating HTML coverage report (unit tests only)..."
	@mkdir -p coverage
	cargo llvm-cov clean --workspace
	cargo llvm-cov --lib --html --output-dir coverage/html
	@echo "✅ HTML coverage report generated in coverage/html/"
	@echo "📊 Open coverage/html/index.html in your browser"

# Clean coverage artifacts
coverage-clean:
	@echo "🧹 Cleaning coverage artifacts..."
	rm -rf coverage/
	cargo llvm-cov clean --workspace
	@echo "✅ Coverage artifacts cleaned"
