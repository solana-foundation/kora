.PHONY: check fmt lint lint-fix test build run clean all regen-tk fix-all generate-ts-client setup-test-env test-integration coverage coverage-all

# Default target
all: check test build

# install
install:
	cargo install --path crates/cli
	cargo install --path crates/rpc

# Check code formatting
check:
	cargo fmt --all -- --check

# Format code
fmt:
	cargo fmt --all

# Run clippy
lint:
	cargo clippy

# Run clippy with auto-fix
lint-fix:
	cargo clippy --fix --allow-dirty
	
# Run tests
test:
	cargo test --lib

# Setup test environment
setup-test-env:
	cargo run -p tests --bin setup-test-env

# Run integration tests
test-integration:
	cargo run -p tests --bin setup-test-env
	cargo test --test integration

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

# Run all fixes and checks
lint-fix-all:
	cargo clippy --fix --allow-dirty -- -D warnings
	cargo fmt --all
	cargo fmt --all -- --check

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

# Generate HTML coverage report (all tests including integration)
coverage-all:
	$(call check_coverage_tool)
	@echo "🧪 Generating HTML coverage report (all tests)..."
	@echo "⚠️  Note: Integration tests may fail if external services aren't available"
	@mkdir -p coverage
	cargo llvm-cov clean --workspace
	cargo llvm-cov --workspace --html --output-dir coverage/html
	@echo "✅ HTML coverage report generated in coverage/html/"
	@echo "📊 Open coverage/html/index.html in your browser"

# Clean coverage artifacts
coverage-clean:
	@echo "🧹 Cleaning coverage artifacts..."
	rm -rf coverage/
	cargo llvm-cov clean --workspace
	@echo "✅ Coverage artifacts cleaned"
