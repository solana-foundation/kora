# Debug test targets - override QUIET_OUTPUT to show all output
debug-test: QUIET_OUTPUT :=
debug-test: test

debug-test-ts: QUIET_OUTPUT :=
debug-test-ts: test-ts
debug-test-integration: QUIET_OUTPUT :=
debug-test-integration: test-integration

debug-test-regular: QUIET_OUTPUT :=
debug-test-regular: test-regular

debug-test-token: QUIET_OUTPUT :=
debug-test-token: test-token

debug-test-auth: QUIET_OUTPUT :=
debug-test-auth: test-auth

debug-test-payment: QUIET_OUTPUT :=
debug-test-payment: test-payment

debug-test-multi-signer: QUIET_OUTPUT :=
debug-test-multi-signer: test-multi-signer

debug-test-all: QUIET_OUTPUT :=
debug-test-all: debug-test debug-test-integration debug-test-ts

.PHONY: debug-test debug-test-ts debug-test-integration debug-test-regular debug-test-token debug-test-auth debug-test-payment debug-test-multi-signer debug-test-all