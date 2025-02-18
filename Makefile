.PHONY: check fmt lint test build run clean all regen-tk fix-all

# Default target
all: check test build

# install
install:
	cargo install --path .

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
docs:
	cargo run -p kora-rpc --bin kora-openapi

# Run all fixes and checks
lint-fix-all:
	cargo clippy --fix -- -D warnings
	cargo fmt --all
	cargo fmt --all -- --check
