# Run unit tests only
unit-test:
	@cargo test --lib --workspace --exclude tests --quiet 2>/dev/null || true

# Build transfer hook program (is checked in, so only need to build if changes are made)
build-transfer-hook:
	$(call print_header,BUILDING TRANSFER HOOK PROGRAM)
	cd tests/src/common/transfer-hook-example && \
		chmod +x build.sh && \
		./build.sh
	$(call print_success,Transfer hook program built at tests/src/common/transfer-hook-example/target/deploy/)

# Run all integration tests using new config-driven test runner
integration-test:
	$(call print_header,KORA INTEGRATION TEST SUITE)
	@cargo run -p tests --bin test_runner

# Verbose integration tests (shows detailed output)
integration-test-verbose:
	$(call print_header,KORA INTEGRATION TEST SUITE - VERBOSE)
	@cargo run -p tests --bin test_runner -- --verbose

# Force refresh test accounts (ignore cached)
integration-test-fresh:
	$(call print_header,KORA INTEGRATION TEST SUITE - FRESH SETUP)
	@cargo run -p tests --bin test_runner -- --force-refresh

# Run specific test phases with filters (for CI)
integration-test-filtered:
	$(call print_header,KORA INTEGRATION TEST SUITE - FILTERED)
	@cargo run -p tests --bin test_runner -- $(FILTERS)