.PHONY: check fmt lint test build run clean all regen-tk fix-all

# Default target
all: check test build

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

# Build release binary
build:
	cargo build --release

# Run presigned release binary
run-presigned:
	cargo run --bin presigned

# Run with default configuration
run:
	cargo run

# Clean build artifacts
clean:
	cargo clean

# Run all fixes and checks
lint-fix-all:
	cargo fmt --all
	cargo clippy --fix -- -D warnings
	cargo fmt --all -- --check
	cargo clippy -- -D warnings
