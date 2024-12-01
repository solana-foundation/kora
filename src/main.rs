mod args;
use core::fmt;
use std::env;

use clap::{Parser, ValueEnum};
use kora::{common, rpc};

#[tokio::main]
async fn main() {
    let args = args::Args::parse();
    setup_metrics(args.metrics_endpoint.clone());
    setup_logging(args.logging_format.clone());

    // TODO : check if signer is already initialized and have a flag for signer option (e.g. tk)

    // log::info!("Initializing signer");

    // let signer = match SolanaMemorySigner::from_base58(&args.private_key) {
    //     Ok(signer) => signer,
    //     Err(e) => {
    //         log::error!("Failed to initialize signer: {}", e);
    //         std::process::exit(1);
    //     }
    // };

    // log::info!("Signer initialized with public key: {}", signer.pubkey_base58());

    // if let Err(e) = common::init_signer(signer) {
    //     log::error!("Failed to initialize signer: {}", e);
    //     std::process::exit(1);
    // }

    log::info!("Starting Kora server");
    log::debug!("Command line arguments: {:?}", args);

    let rpc_client = common::rpc::get_rpc_client(&args.rpc_url);
    log::debug!("RPC client initialized with URL: {}", args.rpc_url);

    let rpc_server = rpc::lib::KoraRpc::new(rpc_client, args.features);
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
