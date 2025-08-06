.PHONY: check lint test build run clean all install generate-key setup-test-env test-integration test-integration-coverage test-all test-ts coverage coverage-clean build-bin build-lib build-rpc build-tk run-presigned openapi gen-ts-client run-metrics

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
	cargo test --lib

# Generate a random key that can be used as an API key or as an HMAC secret
generate-key:
	openssl rand -hex 32

# Server lifecycle management functions
define stop_server
	@echo "🛑 Stopping $(1) server..."
	@pkill -f "kora-rpc.*--port $(TEST_PORT)" || true
	@sleep 2
endef

define start_server
	@echo "🛑 Stopping any existing server..."
	@pkill -f "kora-rpc.*--port $(TEST_PORT)" || true
	@sleep 2
	@echo "🚀 Starting Kora $(1) server..."
	@$(2) -p kora-rpc --bin kora-rpc $(3) -- --private-key $(TEST_PRIVATE_KEY) --config $(4) --rpc-url $(TEST_RPC_URL) --port $(TEST_PORT) $(QUIET_OUTPUT) &
	@echo "⏳ Waiting for server to start..."
	@sleep 5
endef

define run_regular_tests
	@echo "🧪 Running regular integration tests..."
	@$(1) --tests $(2) -- --skip auth_integration_tests $(TEST_OUTPUT_FILTER)
endef

define run_auth_tests
	@echo "🧪 Running auth integration tests..."
	@$(1) --test integration auth_integration_tests $(2) -- --nocapture $(TEST_OUTPUT_FILTER)
endef

define run_integration_phase
	@echo "📋 Phase $(1): $(2)"
	$(call start_server,$(2),$(3),$(4),$(5))
	$(6)
	$(call stop_server,$(2))
endef

# Setup test environment
setup-test-env:
	cargo run -p tests --bin setup-test-env

# Run all integration tests (regular + auth)
test-integration:
	@echo "🧪 Running all integration tests..."
	@echo "🔧 Setting up test environment..."
	@cargo run -p tests --bin setup-test-env $(SETUP_OUTPUT)
	$(call run_integration_phase,1,regular tests,cargo run,,$(REGULAR_CONFIG),$(call run_regular_tests,cargo test,))
	$(call run_integration_phase,2,auth tests,cargo run,,$(AUTH_CONFIG),$(call run_auth_tests,cargo test,))
	@echo "✅ All integration tests completed"

# Run all integration tests with coverage instrumentation (for CI)
test-integration-coverage:
	@echo "🧪 Running all integration tests with coverage..."
	@echo "🔧 Setting up test environment..."
	@cargo run -p tests --bin setup-test-env $(SETUP_OUTPUT)
	$(call run_integration_phase,1,regular tests,cargo llvm-cov run,--no-report,$(REGULAR_CONFIG),$(call run_regular_tests,cargo llvm-cov test,--no-report))
	$(call run_integration_phase,2,auth tests,cargo llvm-cov run,--no-report,$(AUTH_CONFIG),$(call run_auth_tests,cargo llvm-cov test,--no-report))
	@echo "✅ All integration tests completed"


# Run TypeScript SDK tests
test-ts:
	@echo "🧪 Running TypeScript SDK tests..."
	@cd sdks/ts && pnpm test
	@cd sdks/net-ts && pnpm test

test-all: test test-integration test-ts

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
	cargo run -p kora-rpc --bin kora-rpc -- --private-key ./tests/testing-utils/local-keys/fee-payer-local.json --config kora.toml --rpc-url http://127.0.0.1:8899


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

# Run Kora in Docker (no metrics)
run-docker:
	docker compose down
	docker compose build --no-cache kora
	docker compose up

# Update metrics configuration from kora.toml
update-metrics-config:
	@echo "Updating metrics configuration from kora.toml..."
	@cd crates/metrics && cargo run --bin update-config

# Run metrics (Prometheus + Grafana) - automatically updates config first
run-metrics: update-metrics-config
	cd crates/metrics && docker compose -f docker-compose.metrics.yml down
	cd crates/metrics && docker compose -f docker-compose.metrics.yml up
