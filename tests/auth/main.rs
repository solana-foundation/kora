// Authentication tests for Kora RPC server
//
// CONFIG: Uses tests/src/common/fixtures/auth-test.toml (auth enabled)
// TESTS: API key and HMAC authentication middleware
//        - API key authentication via x-api-key header
//        - HMAC authentication via x-timestamp + x-hmac-signature headers
//        - Liveness endpoint bypass (unauthenticated health checks)
//        - Authentication failure scenarios and proper 401 responses

mod api_key_auth;
mod hmac_auth;

// Make common utilities available
#[path = "../src/common/mod.rs"]
mod common;
