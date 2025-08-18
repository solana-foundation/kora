
# Generate TypeScript client
gen-ts-client:
	@echo "🔧 Generating OpenAPI spec with docs feature..."
	cargo run -p kora-cli --bin kora --features docs -- openapi -o openapi.json
	@echo "🚀 Generating TypeScript client..."
	docker run --rm -v "${PWD}:/local" openapitools/openapi-generator-cli generate \
		-i /local/crates/lib/src/rpc_server/openapi/spec/combined_api.json \
		-g typescript-fetch \
		-o /local/generated/typescript-client \
		--additional-properties=supportsES6=true,npmName=kora-client,npmVersion=0.1.0


install-ts-sdk:
	cd sdks/ts && pnpm install

# Build ts sdk
build-ts-sdk:
	cd sdks/ts && pnpm build

# format ts sdk
format-ts-sdk:
	cd sdks/ts && pnpm format