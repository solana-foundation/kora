# Color codes for terminal output
GREEN := \033[0;32m
BLUE := \033[0;34m
BOLD := \033[1m
RESET := \033[0m

# Output control patterns
QUIET_OUTPUT := >/dev/null 2>&1

# Clean structured output functions
define print_header
	@printf "\n$(BOLD)$(BLUE)================================================================================\n"
	@printf "  $(1)\n"
	@printf "================================================================================\n$(RESET)"
endef

define print_success
	@printf "  $(GREEN)✓$(RESET) $(1)\n"
endef

# Generate a random key that can be used as an API key or as an HMAC secret
generate-key:
	@openssl rand -hex 32