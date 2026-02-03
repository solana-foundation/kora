# TypeScript SDK Tests
# NOTE: TypeScript integration tests are now integrated into the main test runner
# Use 'make integration-test' to run all tests including TypeScript phases

unit-test-ts:
	@printf "Running TypeScript SDK unit tests...\n"
	-@cd sdks/ts && pnpm test:unit

test-ts: unit-test-ts
