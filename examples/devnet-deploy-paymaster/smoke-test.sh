#!/usr/bin/env bash
# Verify a live devnet deployer: happy-path deploy lifecycle + adversarial drain/grief probes.
#
#   ./smoke-test.sh                         # full suite, default URL + program
#   ./smoke-test.sh --happy-only            # deploy/verify/close only
#   ./smoke-test.sh --adversarial-only      # probes only
#   ./smoke-test.sh --kora-url <URL>        # override paymaster URL
#   ./smoke-test.sh --program-so <path.so>  # use a different program
#
# Provisions real accounts (spends recoverable devnet SOL) and may leave orphan buffers on failure.
set -euo pipefail
exec cargo run --quiet --manifest-path "$(dirname "$0")/Cargo.toml" --bin devnet_deployer_smoke -- "$@"
