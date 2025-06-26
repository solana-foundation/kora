mod method;
mod rpc;
mod server;
use clap::Parser;
use dotenv::dotenv;
use kora_lib::{
    args::RpcArgs,
    config::load_config,
    log::LoggingFormat,
    rpc::get_rpc_client,
    signer::{init::init_signer_type, KoraSigner},
    state::init_signer,
};
use rpc::KoraRpc;
use server::run_rpc_server;

#[tokio::main]
async fn main() {
    dotenv().ok();
    let args = RpcArgs::parse();
    setup_logging(&args.logging_format);

    let config = load_config(&args.common.config).unwrap_or_else(|e| {
        log::error!("Config load failed: {}", e);
        std::process::exit(1);
    });

    let rpc_client = get_rpc_client(&args.common.rpc_url);

    if let Err(e) = config.validate(rpc_client.as_ref()).await {
        log::error!("Config validation failed: {}", e);
        std::process::exit(1);
    }

    let signer = if !args.common.skip_signer {
        let signer = init_signer_type(&args.common).unwrap();

        // Launch async if privy for init() to populate PublicKey
        match signer {
            KoraSigner::Privy(mut privy_signer) => {
                privy_signer.init().await.unwrap_or_else(|e| {
                    log::error!("Privy signer init failed: {}", e);
                    std::process::exit(1);
                });
                Some(KoraSigner::Privy(privy_signer))
            }
            _ => Some(signer),
        }
    } else {
        None
    };

    if let Some(signer) = signer {
        init_signer(signer).unwrap_or_else(|e| {
            log::error!("Signer init failed: {}", e);
            std::process::exit(1);
        });
    }

    let rpc_server = KoraRpc::new(rpc_client, config.validation, config.kora);
    let server_handle =
        run_rpc_server(rpc_server, args.port.unwrap_or(8080)).await.unwrap_or_else(|e| {
            log::error!("Server start failed: {}", e);
            std::process::exit(1);
        });

    tokio::signal::ctrl_c().await.unwrap();
    server_handle.stop().unwrap();
}

fn setup_logging(format: &LoggingFormat) {
    let env_filter = std::env::var("RUST_LOG")
        .unwrap_or_else(|_| "info,sqlx=error,sea_orm_migration=error,jsonrpsee_server=warn".into());

    let subscriber = tracing_subscriber::fmt().with_env_filter(env_filter);
    match format {
        LoggingFormat::Standard => subscriber.init(),
        LoggingFormat::Json => subscriber.json().init(),
    }
}
