# Include all makefile modules
include makefiles/utils.mk
include makefiles/rs.mk
include makefiles/tests_rs.mk
include makefiles/tests_ts.mk
include makefiles/ts.mk
include makefiles/docs.mk
include makefiles/coverage.mk
include makefiles/metrics.mk

.PHONY: check lint test build run clean all install generate-key setup-test-env test-integration test-integration-coverage test-all test-ts coverage coverage-clean build-bin build-lib build-cli run-presigned openapi gen-ts-client run-metrics

# Default target
all: check test build

# Run all tests (unit + integration + TypeScript)
test-all: test test-integration test-ts

