.PHONY: check fmt lint test build run clean all regen-tk

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

# Run clippy fix
lint-fix:
	cargo clippy -- -D warnings --fix

# Run tests
test:
	cargo test --workspace

# Build release binary
build:
	cargo build --release

# Run with default configuration
run:
	cargo run

# Clean build artifacts
clean:
	cargo clean
