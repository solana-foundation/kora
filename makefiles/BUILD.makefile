# install
install:
	cargo install --path crates/cli --bin kora

# Check code formatting
check:
	cargo fmt --all -- --check

# Run all fixes and checks
lint:
	cargo clippy --fix --allow-dirty -- -D warnings
	cargo fmt --all
	cargo fmt --all -- --check

# Build all binaries
build:
	cargo build --workspace

# Build specific binary
build-bin:
	cargo build --bin $(bin)

# Build lib
build-lib:
	cargo build -p kora-lib

# Build cli
build-cli:
	cargo build -p kora-cli

# Run with default configuration
run:
	cargo run -p kora-cli --bin kora -- --config kora.toml --rpc-url http://127.0.0.1:8899 rpc start --signers-config $(TEST_SIGNERS_CONFIG)

# Clean build artifacts
clean:
	cargo clean


