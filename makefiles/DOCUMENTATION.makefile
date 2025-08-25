# Generate OpenAPI documentation
openapi:
	cargo run -p kora-cli --bin kora --features docs -- openapi -o openapi.json