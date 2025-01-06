mod args;
use args::Args;
use clap::{Parser, ValueEnum};
use common::{load_config, signer::KoraSigner};
use dotenv::dotenv;
use kora::{
    common::{self, tk::TurnkeySigner, vault_signer::VaultSigner, SolanaMemorySigner},
    rpc,
};

#[tokio::main]
async fn main() {
    dotenv().ok();
    let args = args::Args::parse();
    setup_logging(&args.logging_format);

    let config = load_config(&args.config).unwrap_or_else(|e| {
        log::error!("Config load failed: {}", e);
        std::process::exit(1);
    });

    let rpc_client = common::rpc::get_rpc_client(&args.rpc_url);

    if let Err(e) = config.validate(rpc_client.as_ref()).await {
        log::error!("Config validation failed: {}", e);
        std::process::exit(1);
    }

    let signer = if !args.skip_signer { Some(init_signer(&args)) } else { None };

    if let Some(signer) = signer {
        common::init_signer(signer).unwrap_or_else(|e| {
            log::error!("Signer init failed: {}", e);
            std::process::exit(1);
        });
    }

    let rpc_server = rpc::lib::KoraRpc::new(rpc_client, config.validation, config.kora);

    let server_handle =
        rpc::server::run_rpc_server(rpc_server, args.port).await.unwrap_or_else(|e| {
            log::error!("Server start failed: {}", e);
            std::process::exit(1);
        });

    tokio::signal::ctrl_c().await.unwrap();
    server_handle.stop().unwrap();
}

fn init_signer(args: &Args) -> KoraSigner {
    if args.turnkey_signer {
        init_turnkey_signer(args)
    } else if args.vault_signer {
        init_vault_signer(args)
    } else {
        init_memory_signer(args.private_key.as_ref())
    }
}

fn init_vault_signer(args: &Args) -> KoraSigner {
    let vault_addr = args
        .vault_addr
        .as_ref()
        .ok_or_else(|| {
            log::error!("Vault address required");
            std::process::exit(1);
        })
        .unwrap();

    let vault_token = args
        .vault_token
        .as_ref()
        .ok_or_else(|| {
            log::error!("Vault token required");
            std::process::exit(1);
        })
        .unwrap();

    let key_name = args
        .vault_key_name
        .as_ref()
        .ok_or_else(|| {
            log::error!("Vault key name required");
            std::process::exit(1);
        })
        .unwrap();

    let pubkey = args
        .vault_pubkey
        .as_ref()
        .ok_or_else(|| {
            log::error!("Vault public key required");
            std::process::exit(1);
        })
        .unwrap();

    KoraSigner::Vault(
        VaultSigner::new(
            vault_addr.to_string(),
            vault_token.to_string(),
            key_name.to_string(),
            pubkey.to_string(),
        )
        .unwrap_or_else(|e| {
            log::error!("Vault signer init failed: {}", e);
            std::process::exit(1);
        }),
    )
}

fn init_turnkey_signer(args: &Args) -> KoraSigner {
    let api_pub = args
        .turnkey_api_public_key
        .as_ref()
        .ok_or_else(|| {
            log::error!("Turnkey API public key required");
            std::process::exit(1);
        })
        .unwrap();
    let api_priv = args
        .turnkey_api_private_key
        .as_ref()
        .ok_or_else(|| {
            log::error!("Turnkey API private key required");
            std::process::exit(1);
        })
        .unwrap();
    let api_priv_key_id = args
        .turnkey_private_key_id
        .as_ref()
        .ok_or_else(|| {
            log::error!("Turnkey private key ID required");
            std::process::exit(1);
        })
        .unwrap();
    let org_id = args
        .turnkey_organization_id
        .as_ref()
        .ok_or_else(|| {
            log::error!("Turnkey organization ID required");
            std::process::exit(1);
        })
        .unwrap();

    let public_key_id = args
        .turnkey_public_key
        .as_ref()
        .ok_or_else(|| {
            log::error!("Turnkey public key required");
            std::process::exit(1);
        })
        .unwrap();

    KoraSigner::Turnkey(
        TurnkeySigner::new(
            api_pub.to_string(),
            api_priv.to_string(),
            org_id.to_string(),
            api_priv_key_id.to_string(),
            public_key_id.to_string(),
        )
        .unwrap_or_else(|e| {
            log::error!("Turnkey signer init failed: {}", e);
            std::process::exit(1);
        }),
    )
}

fn init_memory_signer(private_key: Option<&String>) -> KoraSigner {
    let key = private_key.unwrap_or_else(|| {
        log::error!("Private key required for memory signer");
        std::process::exit(1);
    });

    KoraSigner::Memory(SolanaMemorySigner::from_base58(key).unwrap_or_else(|e| {
        log::error!("Memory signer init failed: {}", e);
        std::process::exit(1);
    }))
}

#[derive(Parser, Debug, Clone, ValueEnum)]
pub enum LoggingFormat {
    Standard,
    Json,
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
