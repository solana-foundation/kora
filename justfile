set dotenv-load
set quiet

alias b := build
alias t := test
alias ti := test-integration
alias f := fmt
alias r := run

# ******************************************************************************
# Build & Install
# ******************************************************************************

# Default: check, test, build
[group('build')]
default: fmt test build

# Build all workspace packages
[group('build')]
build:
    cargo build --workspace

# Build the lib crate
[group('build')]
build-lib:
    cargo build -p kora-lib

# Build the CLI tool
[group('build')]
build-cli:
    cargo build -p kora-cli

# Build specific binary
[group('build')]
build-bin bin='kora':
    cargo build --bin {{bin}}

# Install kora binary
[group('build')]
install:
    cargo install --path crates/cli --bin kora

# Remove build artifacts
[group('build')]
[confirm('Delete all build artifacts?')]
clean:
    cargo clean

# ******************************************************************************
# Format
# ******************************************************************************

# Check formatting
[group('fmt')]
check:
    cargo fmt --all -- --check
    cargo clippy -- -D warnings

# Format with auto-fix
[group('fmt')]
fmt:
    cargo clippy --fix --allow-dirty -- -D warnings
    cargo fmt --all

# ******************************************************************************
# Testing
# ******************************************************************************

# Run unit tests
[group('test')]
[no-exit-message]
test:
    -cargo test --lib --workspace --exclude tests --quiet

# Run all integration tests (use --verbose, --force-refresh, --filter X as needed)
[group('test')]
test-integration *args: _ensure-transfer-hook
    cargo run -p tests --bin test_runner -- {{args}}

# Run TypeScript SDK unit tests
[group('test')]
[no-exit-message]
test-ts:
    -cd sdks/ts && pnpm test:unit

# Run all tests (unit + TypeScript + integration)
[group('test')]
test-all: build test test-ts test-integration

# Build transfer hook test program
[group('test')]
build-transfer-hook:
    cd tests/src/common/transfer-hook-example && chmod +x build.sh && ./build.sh

[private]
_ensure-transfer-hook:
    #!/usr/bin/env bash
    if [ ! -f "tests/src/common/transfer-hook-example/target/deploy/transfer_hook_example.so" ]; then
        echo "Building transfer hook program..."
        cd tests/src/common/transfer-hook-example && chmod +x build.sh && ./build.sh
    fi

# ******************************************************************************
# Run Services
# ******************************************************************************

# Start RPC server
[group('run')]
run config='kora.toml' rpc='http://127.0.0.1:8899':
    cargo run -p kora-cli --bin kora -- --config {{config}} --rpc-url {{rpc}} rpc start --signers-config tests/src/common/fixtures/signers.toml

# Run Kora in Docker (no metrics)
[group('run')]
run-docker:
    docker compose down
    docker compose build --no-cache kora
    docker compose up

# Run metrics stack (Prometheus + Grafana)
[group('run')]
run-metrics: _update-metrics-config
    cd crates/lib/src/metrics && docker compose -f docker-compose.metrics.yml down
    cd crates/lib/src/metrics && docker compose -f docker-compose.metrics.yml up

[private]
_update-metrics-config:
    cargo run -p kora-lib --bin update-config

# ******************************************************************************
# TypeScript SDK
# ******************************************************************************

# Install TypeScript SDK dependencies
[group('sdk')]
install-ts-sdk:
    cd sdks/ts && pnpm install

# Build TypeScript SDK
[group('sdk')]
build-ts-sdk:
    cd sdks/ts && pnpm build

# Format TypeScript SDK
[group('sdk')]
format-ts-sdk:
    cd sdks/ts && pnpm format

# Generate TypeScript client from OpenAPI
[group('sdk')]
gen-ts-client: openapi
    docker run --rm -v "{{justfile_directory()}}:/local" openapitools/openapi-generator-cli generate \
        -i /local/crates/lib/src/rpc_server/openapi/spec/combined_api.json \
        -g typescript-fetch \
        -o /local/generated/typescript-client \
        --additional-properties=supportsES6=true,npmName=kora-client,npmVersion=0.1.0

# ******************************************************************************
# Documentation & Coverage
# ******************************************************************************

# Generate OpenAPI documentation
[group('docs')]
openapi:
    cargo run -p kora-cli --bin kora --features docs -- openapi -o openapi.json

# Generate HTML coverage report
[group('docs')]
coverage: _ensure-coverage-tools
    mkdir -p coverage
    cargo llvm-cov clean --workspace
    cargo llvm-cov --lib --html --output-dir coverage/html
    @echo "Coverage report: coverage/html/index.html"

# Clean coverage artifacts
[group('docs')]
coverage-clean:
    rm -rf coverage/
    cargo llvm-cov clean --workspace

[private]
_ensure-coverage-tools:
    #!/usr/bin/env bash
    if ! command -v cargo-llvm-cov &>/dev/null; then
        echo "Installing cargo-llvm-cov..."
        cargo install cargo-llvm-cov
    fi
    if ! rustup component list --installed | grep -q llvm-tools-preview; then
        echo "Installing llvm-tools-preview..."
        rustup component add llvm-tools-preview
    fi

# ******************************************************************************
# Utilities
# ******************************************************************************

# Generate a random key (API key or HMAC secret)
[group('util')]
generate-key:
    @openssl rand -hex 32

# ******************************************************************************
# Release
# ******************************************************************************

# Prepare a new release
[group('release')]
[confirm('Start release process?')]
release:
    #!/usr/bin/env bash
    set -euo pipefail

    if [ -n "$(git status --porcelain)" ]; then
        echo "Error: Working directory not clean"
        exit 1
    fi

    command -v cargo-set-version &>/dev/null || { echo "Install cargo-edit: cargo install cargo-edit"; exit 1; }
    command -v git-cliff &>/dev/null || { echo "Install git-cliff: cargo install git-cliff"; exit 1; }

    current=$(cargo metadata --no-deps --format-version 1 | jq -r '.packages[] | select(.name == "kora-lib") | .version')
    echo "Current version: $current"

    read -p "New version: " version
    [ -z "$version" ] && { echo "Version required"; exit 1; }

    echo "Updating to $version..."
    cargo set-version --workspace "$version"

    echo "Generating CHANGELOG..."
    last_tag=$(git tag -l "v*" --sort=-version:refname | head -1)
    if [ -z "$last_tag" ]; then
        git-cliff $(git rev-list --max-parents=0 HEAD)..HEAD --tag "v$version" --config .github/cliff.toml --output CHANGELOG.md --strip all
    elif [ -f CHANGELOG.md ]; then
        git-cliff "$last_tag"..HEAD --tag "v$version" --config .github/cliff.toml --strip all > CHANGELOG.new.md
        cat CHANGELOG.md >> CHANGELOG.new.md
        mv CHANGELOG.new.md CHANGELOG.md
    else
        git-cliff "$last_tag"..HEAD --tag "v$version" --config .github/cliff.toml --output CHANGELOG.md --strip all
    fi

    git add Cargo.toml Cargo.lock CHANGELOG.md crates/*/Cargo.toml

    echo ""
    echo "Ready! Next steps:"
    echo "  git commit -m 'chore: release v$version'"
    echo "  git push origin HEAD"
    echo "  Create PR → merge → trigger 'Publish Rust Crates' workflow"
