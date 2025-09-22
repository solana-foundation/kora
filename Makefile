# Include all makefile modules
include makefiles/UTILS.makefile
include makefiles/BUILD.makefile
include makefiles/RUST_TESTS.makefile
include makefiles/TYPESCRIPT_TESTS.makefile
include makefiles/CLIENT.makefile
include makefiles/DOCUMENTATION.makefile
include makefiles/COVERAGE.makefile
include makefiles/METRICS.makefile

.PHONY: check lint test build run clean all install generate-key setup-test-env test-integration test-all test-ts coverage coverage-clean build-bin build-lib build-cli run-presigned openapi gen-ts-client run-metrics build-transfer-hook

# Default target
all: check test build

# Run all tests (unit + TypeScript + integration)
test-all: test test-ts test-integration