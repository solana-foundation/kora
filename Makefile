.PHONY: check fmt lint test build run clean all regen-tk fix-all generate-ts-client

# Default target
all: check test build

# install
install:
	for dir in crates/*; do \
		if [ -f $$dir/Cargo.toml ]; then \
			cargo install --path $$dir; \
		fi \
	done

# Check code formatting
check:
	cargo fmt --all -- --check

# Format code
fmt:
	cargo fmt --all

# Run clippy
lint:
	cargo clippy -- -D warnings

# Run tests
test:
	cargo test --lib

# Run integration tests
test-integration:
	cargo test --test '*'

# Build all binaries
build:
	cargo build --workspace

# Build specific binary
build-bin:
	cargo build --bin $(bin)

# Build lib
build-lib:
	cargo build -p kora-lib

# Build rpc
build-rpc:
	cargo build -p kora-rpc

# Run presigned release binary
run-presigned:
	cargo run --bin presigned

# Run with default configuration
run:
	cargo run -p kora-rpc

# Clean build artifacts
clean:
	cargo clean

# Gen openapi docs
openapi:
	cargo run -p kora-rpc --bin kora-openapi

# Run all fixes and checks
lint-fix-all:
	cargo clippy --fix -- -D warnings
	cargo fmt --all
	cargo fmt --all -- --check

# Generate TypeScript client
gen-ts-client:
	docker run --rm -v "${PWD}:/local" openapitools/openapi-generator-cli generate \
		-i /local/crates/rpc/src/openapi/spec/combined_api.json \
		-g typescript-fetch \
		-o /local/generated/typescript-client \
		--additional-properties=supportsES6=true,npmName=kora-client,npmVersion=0.1.0
