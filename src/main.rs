mod args;
use core::fmt;
use std::env;

use clap::{Parser, ValueEnum};
use common::load_config;
use kora::{
    common::{
        self, signer::KoraSigner, tk::TurnkeySigner, token::check_valid_tokens, KoraError,
        SolanaMemorySigner,
    },
    rpc,
};
use solana_client::nonblocking::rpc_client::RpcClient;

#[tokio::main]
async fn main() {
    let args = args::Args::parse();
    setup_metrics(args.metrics_endpoint.clone());
    setup_logging(args.logging_format.clone());

    let config = match load_config(args.config.clone()) {
        Ok(config) => config,
        Err(e) => {
            log::error!("Failed to load config: {}", e);
            return;
        }
    };
    log::info!("Config loaded");

    let rpc_client = common::rpc::get_rpc_client(&args.rpc_url);
    log::debug!("RPC client initialized with URL: {}", args.rpc_url);

    if !args.skip_signer {
        let signer = if args.turnkey_signer {
            // Initialize Turnkey signer
            match (
                env::var("TURNKEY_API_PUBLIC_KEY"),
                env::var("TURNKEY_API_PRIVATE_KEY"),
                env::var("TURNKEY_ORGANIZATION_ID"),
                env::var("TURNKEY_EXAMPLE_PRIVATE_KEY_ID"),
            ) {
                (Ok(api_pub), Ok(api_priv), Ok(org_id), Ok(key_id)) => {
                    match TurnkeySigner::new(api_pub, api_priv, org_id, key_id) {
                        Ok(signer) => {
                            log::info!("Turnkey signer initialized");
                            KoraSigner::Turnkey(signer)
                        }
                        Err(e) => {
                            log::error!("Failed to initialize Turnkey signer: {}", e);
                            std::process::exit(1);
                        }
                    }
                }
                _ => {
                    log::error!("Missing required Turnkey environment variables");
                    std::process::exit(1);
                }
            }
        } else {
            // Initialize memory signer
            let private_key = match &args.private_key {
                Some(key) => key,
                None => {
                    log::error!("Private key is required when memory signer is enabled");
                    std::process::exit(1);
                }
            };

            match SolanaMemorySigner::from_base58(private_key) {
                Ok(signer) => {
                    log::info!(
                        "Memory signer initialized with public key: {}",
                        signer.pubkey_base58()
                    );
                    KoraSigner::Memory(signer)
                }
                Err(e) => {
                    log::error!("Failed to initialize memory signer: {}", e);
                    std::process::exit(1);
                }
            }
        };

        if let Err(e) = common::init_signer(signer) {
            log::error!("Failed to initialize signer: {}", e);
            std::process::exit(1);
        }
    }

    let rpc_server = rpc::lib::KoraRpc::new(rpc_client, config.validation, config.kora);
    log::debug!("RPC server instance created");

    log::info!("Attempting to start RPC server on port {}", args.port);
    let server_handle = match rpc::server::run_rpc_server(rpc_server, args.port).await {
        Ok(handle) => {
            log::info!("Server started successfully");
            handle
        }
        Err(e) => {
            log::error!("Failed to start server: {}", e);
            return;
        }
    };

    log::info!("Server running. Press Ctrl+C to stop");
    tokio::signal::ctrl_c().await.unwrap();
    log::info!("Shutting down server");
    server_handle.stop().unwrap();
    log::info!("Server stopped");
}

fn setup_metrics(endpoint: Option<String>) {
    if let Some(endpoint) = endpoint {
        log::info!("Metrics endpoint: {}", endpoint);
    }
}

#[derive(Parser, Debug, Clone, ValueEnum)]
pub enum LoggingFormat {
    Standard,
    Json,
}

impl fmt::Display for LoggingFormat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LoggingFormat::Standard => write!(f, "standard"),
            LoggingFormat::Json => write!(f, "json"),
        }
    }
}

pub fn setup_logging(logging_format: LoggingFormat) {
    let env_filter = env::var("RUST_LOG")
        .unwrap_or("info,sqlx=error,sea_orm_migration=error,jsonrpsee_server=warn".to_string());
    let subscriber = tracing_subscriber::fmt().with_env_filter(env_filter);
    match logging_format {
        LoggingFormat::Standard => subscriber.init(),
        LoggingFormat::Json => subscriber.json().init(),
    }
}

pub async fn validate_config(
    config: &common::config::Config,
    rpc_client: RpcClient,
) -> Result<(), KoraError> {
    if config.validation.allowed_tokens.is_empty() {
        log::error!("No tokens enabled");
        return Err(KoraError::InternalServerError("No tokens enabled".to_string()));
    }

    check_valid_tokens(&rpc_client, &config.validation.allowed_tokens).await?;
    Ok(())
}
