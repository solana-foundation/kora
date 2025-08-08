mod args;

use args::GlobalArgs;
use clap::{Parser, Subcommand};
use kora_lib::{
    error::KoraError,
    rpc::get_rpc_client,
    rpc_server::{run_rpc_server, KoraRpc, RpcArgs},
    signer::init::init_signer_type,
    state::init_signer,
    validator::config_validator::ConfigValidator,
    Config,
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
    /// Start the RPC server
    Rpc {
        #[command(flatten)]
        rpc_args: Box<RpcArgs>,
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
    Validate,
    /// Validate configuration file with RPC validation (slower but more thorough)
    ValidateWithRpc,
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
    let cli = Cli::parse();

    let config = Config::load_config(&cli.global_args.config).unwrap_or_else(|e| {
        print_error(&format!("Failed to load config: {e}"));
        std::process::exit(1);
    });

    let rpc_client = get_rpc_client(&cli.global_args.rpc_url);

    match cli.command {
        Some(Commands::Config { config_command }) => {
            match config_command {
                ConfigCommands::Validate => {
                    let _ =
                        ConfigValidator::validate_with_result(&config, rpc_client.as_ref(), true)
                            .await;
                }
                ConfigCommands::ValidateWithRpc => {
                    let _ =
                        ConfigValidator::validate_with_result(&config, rpc_client.as_ref(), false)
                            .await;
                }
            }
            std::process::exit(0);
        }
        Some(Commands::Rpc { rpc_args }) => {
            // Validate config before starting server
            if let Err(e) = ConfigValidator::validate(&config, rpc_client.as_ref()).await {
                print_error(&format!("Config validation failed: {e}"));
                std::process::exit(1);
            }

            // Initialize the signer
            if !rpc_args.skip_signer {
                let signer = init_signer_type(&rpc_args).unwrap();
                init_signer(signer).unwrap_or_else(|e| {
                    print_error(&format!("Failed to initialize signer: {e}"));
                    std::process::exit(1);
                });
            }

            let rpc_client = get_rpc_client(&cli.global_args.rpc_url);

            let kora_rpc = KoraRpc::new(rpc_client, config.validation, config.kora);

            let _server_handle = run_rpc_server(kora_rpc, rpc_args.port).await?;

            tokio::signal::ctrl_c().await.unwrap();
            println!("Shutting down server...");
        }
        #[cfg(feature = "docs")]
        Some(Commands::Openapi { output }) => {
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
            println!("  config validate         - Validate configuration");
            println!("  config validate-with-rpc - Validate configuration with RPC calls");
            println!("  rpc                     - Start RPC server");
            #[cfg(feature = "docs")]
            println!("  openapi                 - Generate OpenAPI documentation");
        }
    }

    Ok(())
}

fn print_error(message: &str) {
    eprintln!("Error: {message}");
}
