mod args;

use args::GlobalArgs;
use clap::{Parser, Subcommand};
use kora_lib::{
    admin::token_util::initialize_atas,
    error::KoraError,
    log::LoggingFormat,
    rpc::get_rpc_client,
    rpc_server::{run_rpc_server, server::ServerHandles, KoraRpc, RpcArgs},
    signer::init::init_signers,
    state::init_config,
    validator::config_validator::ConfigValidator,
    CacheUtil, Config,
};

#[cfg(feature = "docs")]
use kora_lib::rpc_server::openapi::docs;
#[cfg(feature = "docs")]
use utoipa::OpenApi;

#[derive(Subcommand)]
enum Commands {
    /// Configuration management commands
    Config {
        #[command(subcommand)]
        config_command: ConfigCommands,
    },
    /// RPC server operations
    Rpc {
        #[command(subcommand)]
        rpc_command: RpcCommands,
    },
    /// Generate OpenAPI documentation
    #[cfg(feature = "docs")]
    Openapi {
        /// Output path for the OpenAPI spec file
        #[arg(short = 'o', long, default_value = "openapi.json")]
        output: String,
    },
}

#[derive(Subcommand)]
enum ConfigCommands {
    /// Validate configuration file (fast, no RPC calls)
    Validate {
        /// Path to signers configuration file (optional)
        #[arg(long)]
        signers_config: Option<std::path::PathBuf>,
    },
    /// Validate configuration file with RPC validation (slower but more thorough)
    ValidateWithRpc {
        /// Path to signers configuration file (optional)
        #[arg(long)]
        signers_config: Option<std::path::PathBuf>,
    },
}

#[derive(Subcommand)]
enum RpcCommands {
    /// Start the RPC server
    #[command(
        about = "Start the RPC server",
        long_about = "Start the Kora RPC server to handle gasless transactions.\n\nThe server will validate the configuration and initialize the specified signer before starting."
    )]
    Start {
        #[command(flatten)]
        rpc_args: Box<RpcArgs>,
    },
    /// Initialize ATAs for all allowed payment tokens
    #[command(
        about = "Initialize ATAs for all allowed payment tokens",
        long_about = "Initialize Associated Token Accounts (ATAs) for all payment tokens configured in the system.\n\nThis command creates ATAs in the following priority order:\n1. If a payment address is configured, creates ATAs for that address only\n2. Otherwise, creates ATAs for ALL signers in the pool\n\nYou can specify which signer to use as the fee payer for the ATA creation transactions.\nIf no fee payer is specified, the first signer in the pool will be used."
    )]
    InitializeAtas {
        #[command(flatten)]
        rpc_args: Box<RpcArgs>,

        /// Signer key to use as fee payer (defaults to first signer if not specified)
        #[arg(long, help_heading = "Signer Options")]
        fee_payer_key: Option<String>,

        /// Compute unit price for priority fees (in micro-lamports)
        #[arg(long, help_heading = "Transaction Options")]
        compute_unit_price: Option<u64>,

        /// Compute unit limit for transactions
        #[arg(long, help_heading = "Transaction Options")]
        compute_unit_limit: Option<u32>,

        /// Number of ATAs to create per transaction
        #[arg(long, help_heading = "Transaction Options")]
        chunk_size: Option<usize>,
    },
}

#[derive(Parser)]
#[command(author, version, about = "Kora - Solana gasless transaction relayer", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    #[command(flatten)]
    pub global_args: GlobalArgs,
}

#[tokio::main]
async fn main() -> Result<(), KoraError> {
    dotenv::dotenv().ok();
    let cli = Cli::parse();

    let config = Config::load_config(&cli.global_args.config).unwrap_or_else(|e| {
        print_error(&format!("Failed to load config: {e}"));
        std::process::exit(1);
    });

    init_config(config).unwrap_or_else(|e| {
        print_error(&format!("Failed to initialize config: {e}"));
        std::process::exit(1);
    });

    let rpc_client = get_rpc_client(&cli.global_args.rpc_url);

    match cli.command {
        Some(Commands::Config { config_command }) => {
            match config_command {
                ConfigCommands::Validate { signers_config } => {
                    let _ = ConfigValidator::validate_with_result_and_signers(
                        rpc_client.as_ref(),
                        true,
                        signers_config.as_ref(),
                    )
                    .await;
                }
                ConfigCommands::ValidateWithRpc { signers_config } => {
                    let _ = ConfigValidator::validate_with_result_and_signers(
                        rpc_client.as_ref(),
                        false,
                        signers_config.as_ref(),
                    )
                    .await;
                }
            }
            std::process::exit(0);
        }
        Some(Commands::Rpc { rpc_command }) => {
            match rpc_command {
                RpcCommands::Start { rpc_args } => {
                    // Validate config and signers before starting server
                    match ConfigValidator::validate_with_result_and_signers(
                        rpc_client.as_ref(),
                        true,
                        rpc_args.signers_config.as_ref(),
                    )
                    .await
                    {
                        Err(errors) => {
                            for e in errors {
                                print_error(&format!("Validation error: {e}"));
                            }
                            std::process::exit(1);
                        }
                        Ok(warnings) => {
                            for w in warnings {
                                println!("Warning: {w}");
                            }
                        }
                    }

                    setup_logging(&rpc_args.logging_format);

                    // Initialize signer(s) - supports both single and multi-signer modes
                    if !rpc_args.skip_signer {
                        init_signers(&rpc_args).await.unwrap_or_else(|e| {
                            print_error(&format!("Failed to initialize signer(s): {e}"));
                            std::process::exit(1);
                        });
                    }

                    // Initialize cache
                    if let Err(e) = CacheUtil::init().await {
                        print_error(&format!("Failed to initialize cache: {e}"));
                        std::process::exit(1);
                    }

                    let rpc_client = get_rpc_client(&cli.global_args.rpc_url);

                    let kora_rpc = KoraRpc::new(rpc_client);

                    let ServerHandles { rpc_handle, metrics_handle, balance_tracker_handle } =
                        run_rpc_server(kora_rpc, rpc_args.port).await?;

                    tokio::signal::ctrl_c().await.unwrap();
                    println!("Shutting down server...");

                    // Stop the balance tracker task
                    if let Some(handle) = balance_tracker_handle {
                        log::info!("Stopping balance tracker background task...");
                        handle.abort();
                    }

                    // Stop the RPC server
                    rpc_handle.stop().unwrap();

                    // Stop the metrics server if running
                    if let Some(handle) = metrics_handle {
                        handle.stop().unwrap();
                    }
                }
                RpcCommands::InitializeAtas {
                    rpc_args,
                    fee_payer_key,
                    compute_unit_price,
                    compute_unit_limit,
                    chunk_size,
                } => {
                    if !rpc_args.skip_signer {
                        init_signers(&rpc_args).await.unwrap_or_else(|e| {
                            print_error(&format!("Failed to initialize signer(s): {e}"));
                            std::process::exit(1);
                        });
                    } else {
                        print_error("Cannot initialize ATAs without a signer.");
                        std::process::exit(1);
                    }

                    // Initialize cache
                    if let Err(e) = CacheUtil::init().await {
                        print_error(&format!("Failed to initialize cache: {e}"));
                        std::process::exit(1);
                    }

                    // Initialize ATAs
                    if let Err(e) = initialize_atas(
                        rpc_client.as_ref(),
                        compute_unit_price,
                        compute_unit_limit,
                        chunk_size,
                        fee_payer_key,
                    )
                    .await
                    {
                        print_error(&format!("Failed to initialize ATAs: {e}"));
                        std::process::exit(1);
                    }
                    println!("Successfully initialized all payment ATAs");
                }
            }
        }

        #[cfg(feature = "docs")]
        Some(Commands::Openapi { output }) => {
            docs::update_docs();

            let openapi_spec = docs::ApiDoc::openapi();
            let json = serde_json::to_string_pretty(&openapi_spec).unwrap_or_else(|e| {
                print_error(&format!("Failed to serialize OpenAPI spec: {e}"));
                std::process::exit(1);
            });

            std::fs::write(&output, json).unwrap_or_else(|e| {
                print_error(&format!("Failed to write OpenAPI spec to {}: {e}", output));
                std::process::exit(1);
            });

            println!("OpenAPI spec written to: {}", output);
        }
        None => {
            println!("No command specified. Use --help for usage information.");
            println!("Available commands:");
            println!("  config validate          - Validate configuration");
            println!("  config validate-with-rpc - Validate configuration with RPC calls");
            println!("  rpc start                - Start RPC server");
            println!("  rpc initialize-atas      - Initialize ATAs for payment tokens");
            #[cfg(feature = "docs")]
            println!("  openapi                  - Generate OpenAPI documentation");
        }
    }

    Ok(())
}

fn print_error(message: &str) {
    eprintln!("Error: {message}");
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
