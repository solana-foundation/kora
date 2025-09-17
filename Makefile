# Include all makefile modules
include makefiles/UTILS.makefile
include makefiles/BUILD.makefile
include makefiles/RUST_TESTS.makefile
include makefiles/TYPESCRIPT_TESTS.makefile
include makefiles/CLIENT.makefile
include makefiles/DOCUMENTATION.makefile
include makefiles/COVERAGE.makefile
include makefiles/METRICS.makefile
include makefiles/DEBUG.makefile

.PHONY: check lint test build run clean all install generate-key setup-test-env test-integration test-all test-ts coverage coverage-clean build-bin build-lib build-cli run-presigned openapi gen-ts-client run-metrics build-transfer-hook debug-test debug-test-ts debug-test-integration debug-test-regular debug-test-token debug-test-auth debug-test-payment debug-test-multi-signer debug-test-all

# Default target
all: check test build

# Run all tests (unit + integration + TypeScript)
test-all: test test-integration test-ts