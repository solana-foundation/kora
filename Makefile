.PHONY: check fmt lint test build run clean all

# Default target
all: check test build

# Check formatting
fmt:
	cargo fmt --all -- --check

# Run clippy lints
lint:
	cargo clippy --all-targets --all-features -- -D warnings

# Run tests
test:
	cargo test --all-features

# Build release binary
build:
	cargo build --release

# Run with default configuration
run:
	cargo run

# Check everything without building
check: fmt lint
	cargo check --all-targets --all-features

# Clean build artifacts
clean:
	cargo clean

# Watch tests
watch-test:
	cargo watch -x test

# Watch build
watch:
	cargo watch -x build

# Format code
format:
	cargo fmt --all

# Development setup
setup:
	rustup component add clippy rustfmt
	cargo install cargo-watch
