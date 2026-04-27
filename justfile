set dotenv-load
set quiet

alias b := build
alias t := unit-test
alias ti := integration-test
alias f := fmt
alias r := run

# ******************************************************************************
# Build & Install
# ******************************************************************************

# Default: check, test, build
[group('build')]
default: fmt unit-test build

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
    cd sdks/ts && pnpm format:check && pnpm type-check

# Format with auto-fix
[group('fmt')]
fmt:
    cargo clippy --fix --allow-dirty -- -D warnings
    cargo fmt --all
    cd sdks/ts && pnpm format

# Format TypeScript Files
fmt-ts:
    cd sdks/ts && pnpm format

# ******************************************************************************
# Testing
# ******************************************************************************

# Run unit tests
[group('test')]
[no-exit-message]
unit-test:
    -cargo test --lib --workspace --exclude tests --quiet

# Run all integration tests (use --verbose, --force-refresh, --filter X as needed)
[group('test')]
integration-test *args: build _ensure-transfer-hook
    cargo run -p tests --bin test_runner -- {{args}}

# Run TypeScript SDK unit tests
[group('test')]
[no-exit-message]
unit-test-ts: build
    -cd sdks/ts && pnpm test:unit

# Run all TypeScript SDK tests (unit + integration)
[group('test')]
[no-exit-message]
test-ts: build _ensure-transfer-hook
    cd sdks/ts && pnpm test:unit
    cargo run -p tests --bin test_runner -- --filter typescript_basic
    cargo run -p tests --bin test_runner -- --filter typescript_auth
    cargo run -p tests --bin test_runner -- --filter typescript_free

# Run all tests (unit + TypeScript + integration)
[group('test')]
test: build unit-test unit-test-ts integration-test

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

# Show branch workflow guidance
[group('release')]
branch-info:
    @echo "Branch Workflow:"
    @echo "  main                → Integration branch (audited + unaudited commits)"
    @echo "  feat/*,fix/*,chore/* → Topic branches from main"
    @echo "  hotfix/*            → Urgent fixes from deployed stable tag"
    @echo ""
    @echo "Releasing:"
    @echo "  Stable/Beta/RC: checkout main, run 'just release'"
    @echo "  Hotfix patch: run 'just release' from hotfix/*"
    @echo "  Pre-release versions use semver suffixes (e.g. 2.3.0-beta.1)"
    @echo "  Hotfix: run 'just hotfix' from deployed stable tag"

# Prepare a new release (run from main or hotfix/*; use semver pre-release suffixes for beta/rc)
[group('release')]
[confirm('Start release process?')]
release:
    #!/usr/bin/env bash
    set -euo pipefail

    if [ -n "$(git status --porcelain)" ]; then
        echo "Error: Working directory not clean"
        exit 1
    fi

    current_branch=$(git rev-parse --abbrev-ref HEAD)
    case "$current_branch" in
        main|hotfix/*) ;;
        *)
            echo "Error: Releases must be prepared from main or hotfix/* (current branch: $current_branch)"
            exit 1
            ;;
    esac

    command -v cargo-set-version &>/dev/null || { echo "Install cargo-edit: cargo install cargo-edit"; exit 1; }
    command -v git-cliff &>/dev/null || { echo "Install git-cliff: cargo install git-cliff"; exit 1; }

    current=$(cargo metadata --no-deps --format-version 1 | jq -r '.packages[] | select(.name == "kora-lib") | .version')
    echo "Current version: $current"

    read -p "New version: " version
    [ -z "$version" ] && { echo "Version required"; exit 1; }

    echo "Updating to $version..."
    cargo set-version --workspace "$version"

    # Update kora-lib version in workspace.dependencies
    sed -i.bak "s/kora-lib = { path = \"crates\/lib\", version = \"[^\"]*\" }/kora-lib = { path = \"crates\/lib\", version = \"$version\" }/" Cargo.toml
    rm -f Cargo.toml.bak

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
    if [[ "$current_branch" == hotfix/* ]]; then
        echo "  Trigger 'Publish Rust Crates' workflow from this hotfix branch"
        echo "  Trigger 'Publish TypeScript SDK' workflow from this hotfix branch (if needed)"
        echo "  Then merge hotfix back to main"
    else
        echo "  Create PR → merge → trigger 'Publish Rust Crates' workflow"
    fi

# Start a hotfix branch from a deployed stable tag
[group('release')]
hotfix name='' base_tag='':
    #!/usr/bin/env bash
    set -euo pipefail

    if [ -z "{{name}}" ]; then
        read -p "Hotfix branch name (without 'hotfix/'): " name
        [ -z "$name" ] && { echo "Name required"; exit 1; }
    else
        name="{{name}}"
    fi

    name="${name#hotfix/}"
    branch_name="hotfix/$name"

    git fetch --tags origin

    latest_tag=$(git tag -l "v*" --sort=-version:refname | head -1)
    if [ -z "{{base_tag}}" ]; then
        read -p "Base deployed tag [$latest_tag]: " base_tag
        base_tag="${base_tag:-$latest_tag}"
    else
        base_tag="{{base_tag}}"
    fi

    if [ -z "$base_tag" ]; then
        echo "Error: Base tag required"
        exit 1
    fi

    if ! git rev-parse -q --verify "refs/tags/$base_tag" >/dev/null; then
        if [[ "$base_tag" != v* ]] && git rev-parse -q --verify "refs/tags/v$base_tag" >/dev/null; then
            base_tag="v$base_tag"
        else
            echo "Error: Tag '$base_tag' not found"
            exit 1
        fi
    fi

    # Check if branch already exists
    if git show-ref --verify --quiet "refs/heads/$branch_name"; then
        echo "Branch $branch_name already exists"
        read -p "Switch to it? [y/N] " switch
        if [[ "$switch" =~ ^[Yy]$ ]]; then
            git checkout "$branch_name"
        fi
    elif git show-ref --verify --quiet "refs/remotes/origin/$branch_name"; then
        echo "Remote branch origin/$branch_name already exists"
        read -p "Create local tracking branch? [y/N] " track
        if [[ "$track" =~ ^[Yy]$ ]]; then
            git checkout -b "$branch_name" --track "origin/$branch_name"
        fi
    else
        read -p "Create branch $branch_name from tag $base_tag? [y/N] " create
        if [[ "$create" =~ ^[Yy]$ ]]; then
            git checkout -b "$branch_name" "$base_tag"
            echo ""
            echo "Created $branch_name from tag $base_tag"
        else
            echo "Aborted"
            exit 0
        fi
    fi

    echo ""
    echo "Next steps:"
    echo "  1. Apply your hotfix commits"
    echo "  2. Run 'just release' on this hotfix branch"
    echo "  3. Trigger publish workflows from this hotfix branch"
    echo "  4. Push and merge hotfix back to main"

# Prepare a new TypeScript SDK release
[group('release')]
[confirm('Start TS SDK release process?')]
release-ts-sdk:
    #!/usr/bin/env bash
    set -euo pipefail

    if [ -n "$(git status --porcelain)" ]; then
        echo "Error: Working directory not clean"
        exit 1
    fi

    current=$(node -p "require('./sdks/ts/package.json').version")
    echo "Current version: $current"

    read -p "New version: " version
    [ -z "$version" ] && { echo "Version required"; exit 1; }

    echo "Updating to $version..."
    npm version "$version" --no-git-tag-version --prefix sdks/ts

    git add sdks/ts/package.json

    echo ""
    echo "Ready! Next steps:"
    echo "  git commit -m 'chore: release ts-sdk v$version'"
    echo "  git push origin HEAD"
    echo "  Trigger 'Publish TypeScript SDK (Manual)' workflow"

# ******************************************************************************
# Deploy
# ******************************************************************************

# Redeploy the devnet paymaster example to GCP Cloud Run. Re-derives the KMS key path,
# the Solana pubkey from the KMS public key, and the Memorystore Redis URL — so this
# tracks whatever resources actually exist in the configured GCP project.
#
# Requires:
#   - gcloud authed against the project hosting the paymaster
#   - python with `cryptography` + `base58` available (sources $VENV/bin/activate if present)
#
# Override defaults via env vars: REGION, KEYRING, KEY, MEMORYSTORE_NAME, CONNECTOR_NAME,
# SERVICE_NAME, RPC_URL, VENV.
[group('deploy')]
deploy-devnet-paymaster:
    #!/usr/bin/env bash
    set -euo pipefail
    : "${REGION:=us-central1}"
    : "${KEYRING:=kora-devnet}"
    : "${KEY:=paymaster}"
    : "${MEMORYSTORE_NAME:=kora-devnet-cache}"
    : "${CONNECTOR_NAME:=kora-vpc-connector}"
    : "${SERVICE_NAME:=kora-devnet-paymaster}"
    : "${RPC_URL:=https://api.devnet.solana.com}"
    : "${VENV:=$HOME/.venv_global}"

    if [ -f "$VENV/bin/activate" ]; then
        # shellcheck disable=SC1090
        source "$VENV/bin/activate"
    fi

    KEY_NAME=$(gcloud kms keys versions list \
        --key "$KEY" --keyring "$KEYRING" --location global \
        --format='value(name)' --limit 1)
    REDIS_HOST=$(gcloud redis instances describe "$MEMORYSTORE_NAME" \
        --region="$REGION" --format='value(host)')
    REDIS_PORT=$(gcloud redis instances describe "$MEMORYSTORE_NAME" \
        --region="$REGION" --format='value(port)')
    KORA_REDIS_URL="redis://$REDIS_HOST:$REDIS_PORT"

    PUB_PEM=$(mktemp)
    trap 'rm -f "$PUB_PEM"' EXIT
    gcloud kms keys versions get-public-key 1 \
        --key "$KEY" --keyring "$KEYRING" --location global > "$PUB_PEM"
    KORA_PUBKEY=$(python3 -c "
    from cryptography.hazmat.primitives.serialization import load_pem_public_key
    import base58
    pk = load_pem_public_key(open('$PUB_PEM','rb').read()).public_bytes_raw()
    print(base58.b58encode(pk).decode())
    ")

    echo "=== Cloud Run deploy ==="
    echo "  Service:   $SERVICE_NAME"
    echo "  Region:    $REGION"
    echo "  KMS:       $KEY_NAME"
    echo "  Pubkey:    $KORA_PUBKEY"
    echo "  Redis:     $KORA_REDIS_URL"

    gcloud run deploy "$SERVICE_NAME" \
        --source examples/devnet-deploy-paymaster/ \
        --region "$REGION" \
        --allow-unauthenticated \
        --port 8080 \
        --memory 1Gi \
        --vpc-connector "$CONNECTOR_NAME" \
        --vpc-egress private-ranges-only \
        --set-env-vars="RPC_URL=$RPC_URL,KORA_GCP_KMS_KEY_NAME=$KEY_NAME,KORA_GCP_KMS_PUBLIC_KEY=$KORA_PUBKEY,KORA_REDIS_URL=$KORA_REDIS_URL"
