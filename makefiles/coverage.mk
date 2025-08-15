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

# Run all integration tests with coverage instrumentation (for CI)
test-integration-coverage:
	@echo "🧪 Running all integration tests with coverage..."
	@echo "🔧 Setting up test environment..."
	@cargo run -p tests --bin setup_test_env $(SETUP_OUTPUT)
	$(call run_integration_phase,1,regular tests,$(REGULAR_CONFIG),true,--test integration)
	$(call run_integration_phase,2,auth tests,$(AUTH_CONFIG),,--test auth)
	$(call run_integration_phase,3,payment address tests,$(PAYMENT_ADDRESS_CONFIG),,--test payment-address)
	@echo "✅ All integration tests completed"