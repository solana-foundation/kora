
# Debug test convenience targets
debug-test-integration:
	@make -f makefiles/DEBUG.makefile debug-test-integration

debug-test-regular:
	@make -f makefiles/DEBUG.makefile debug-test-regular

debug-test-token:
	@make -f makefiles/DEBUG.makefile debug-test-token

debug-test-auth:
	@make -f makefiles/DEBUG.makefile debug-test-auth

debug-test-payment:
	@make -f makefiles/DEBUG.makefile debug-test-payment

debug-test-multi-signer:
	@make -f makefiles/DEBUG.makefile debug-test-multi-signer

debug-test-all:
	@make -f makefiles/DEBUG.makefile debug-test-all

.PHONY: debug-test-integration debug-test-regular debug-test-token debug-test-auth debug-test-payment debug-test-multi-signer debug-test-all
