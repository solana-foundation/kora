# TypeScript SDK Tests
# NOTE: TypeScript integration tests are now integrated into the main test runner
# Use 'make test-integration' to run all tests including TypeScript phases

test-ts-unit:
	@printf "Running TypeScript SDK unit tests...\n"
	-@cd sdks/ts && pnpm test:unit

test-ts: test-ts-unit
