#!/usr/bin/env bash
# End-to-end smoke test against the live devnet paymaster.
#
# Deploys a real program through Kora (transfer-hook-example.so by default),
# verifies on-chain that Kora is the upgrade authority, then closes the program
# to recover rent. Self-cleaning, zero net SOL cost.
#
#   ./smoke-test.sh                         # default URL + program
#   ./smoke-test.sh --kora-url <URL>        # override paymaster URL
#   ./smoke-test.sh --program-so <path.so>  # deploy a different program

set -euo pipefail
exec cargo run --quiet --manifest-path "$(dirname "$0")/Cargo.toml" -- "$@"
