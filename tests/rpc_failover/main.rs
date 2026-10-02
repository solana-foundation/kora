// RPC Failover Integration Tests
//
// CONFIG: Uses tests/src/common/fixtures/kora-test.toml (no auth enabled)
// RPC: Kora is started with `--rpc-urls <unreachable>,<surfnet>`, so the preferred
//     endpoint is dead from the very first request and every RPC call the server
//     makes has to fail over to the live one.
// TESTS: That a node whose primary RPC endpoint is unreachable still serves every
//        method, including the ones that reach deep into fee estimation and token
//        validation with nothing but a `&RpcClient` in hand.

mod failover;

#[path = "../src/common/mod.rs"]
mod common;

use common::{failover_harness_context, KoraSpec, TestContext};

/// A context against a node whose primary RPC endpoint refuses connections.
pub async fn ctx() -> TestContext {
    failover_harness_context(KoraSpec {
        config: "tests/src/common/fixtures/kora-test.toml",
        signers: "tests/src/common/fixtures/signers.toml",
        initialize_payments_atas: false,
    })
    .await
}
