use crate::{
    config::{EnabledMethods, FeePayerPolicy, ValidationConfig},
    fee::price::{PriceConfig, PriceModel},
    oracle::oracle::{PriceSource, TokenPrice},
};
use std::path::PathBuf;
use utoipa::{
    openapi::{
        path::OperationBuilder, request_body::RequestBodyBuilder, ContentBuilder, PathItem,
        PathItemType, Required, ResponseBuilder, ResponsesBuilder, ServerBuilder,
    },
    OpenApi,
};

use crate::rpc_server::{
    method::{
        get_blockhash::GetBlockhashResponse,
        get_config::GetConfigResponse,
        get_payer_signer::GetPayerSignerResponse,
        get_supported_tokens::GetSupportedTokensResponse,
        sign_and_send_transaction::{
            SignAndSendTransactionRequest, SignAndSendTransactionResponse,
        },
        sign_transaction::{SignTransactionRequest, SignTransactionResponse},
        sign_transaction_if_paid::{SignTransactionIfPaidRequest, SignTransactionIfPaidResponse},
        transfer_transaction::{TransferTransactionRequest, TransferTransactionResponse},
    },
    KoraRpc,
};

use super::helper::{build_error_response, request_schema};

const JSON_CONTENT_TYPE: &str = "application/json";

#[derive(OpenApi)]
#[openapi(
    info(
        title = "kora-rpc",
        version = "0.1.0",
        description = "RPC server for Kora gasless relayer",
        license(name = "MIT")
    ),
    components(schemas(
        ValidationConfig,
        FeePayerPolicy,
        EnabledMethods,
        PriceConfig,
        PriceModel,
        TokenPrice,
        PriceSource,
        GetBlockhashResponse,
        GetConfigResponse,
        GetPayerSignerResponse,
        GetSupportedTokensResponse,
        SignAndSendTransactionRequest,
        SignAndSendTransactionResponse,
        SignTransactionRequest,
        SignTransactionResponse,
        SignTransactionIfPaidRequest,
        SignTransactionIfPaidResponse,
        TransferTransactionRequest,
        TransferTransactionResponse,
    ))
)]
pub struct ApiDoc;

pub fn update_docs() {
    let method_specs = KoraRpc::build_docs_spec();
    let mut combined_doc = ApiDoc::openapi();

    // Get base components
    let components = combined_doc.components.unwrap_or_default();

    combined_doc.servers =
        Some(vec![ServerBuilder::new().url("https://api.example.com/v1".to_string()).build()]);

    for spec in method_specs {
        let content =
            ContentBuilder::new().schema(request_schema(&spec.name, spec.request.clone())).build();

        let request_body = RequestBodyBuilder::new()
            .content(JSON_CONTENT_TYPE, content)
            .required(Some(Required::True))
            .build();

        let responses = ResponsesBuilder::new()
            .response(
                "200",
                ResponseBuilder::new().description("Successful response").content(
                    JSON_CONTENT_TYPE,
                    ContentBuilder::new().schema(spec.response.clone()).build(),
                ),
            )
            .response("429", build_error_response("Exceeded rate limit."))
            .response("500", build_error_response("Internal server error."))
            .build();

        let operation =
            OperationBuilder::new().request_body(Some(request_body)).responses(responses).build();

        let mut path_item = PathItem::new(PathItemType::Post, operation);
        path_item.summary = Some(spec.name.clone());

        combined_doc.paths.paths.insert(format!("/{}", spec.name), path_item);
    }

    // Set the components
    combined_doc.components = Some(components);

    let json = serde_json::to_string_pretty(&combined_doc).unwrap();
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("src/rpc_server/openapi/spec/combined_api.json");

    // Create parent directory if it doesn't exist
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).unwrap();
    }

    std::fs::write(&path, json).unwrap();

    let validate_result = std::process::Command::new("npx")
        .arg("@redocly/cli@latest")
        .arg("lint")
        .arg("--extends=minimal")
        .arg(path.to_str().unwrap())
        .output()
        .unwrap();

    if !validate_result.status.success() {
        let stderr = String::from_utf8_lossy(&validate_result.stderr);
        panic!("Failed to validate OpenAPI schema: {stderr}");
    }
}
