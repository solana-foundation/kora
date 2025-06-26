use clap::{Parser, Subcommand};
use kora_lib::{
    args::CliArgs,
    config::load_config,
    error::KoraError,
    rpc::{create_rpc_client, get_rpc_client},
    signer::init::init_signer_type,
    state::init_signer,
    transaction::{
        decode_b64_transaction, estimate_transaction_fee, sign_and_send_transaction,
        sign_transaction, sign_transaction_if_paid,
    },
};

#[derive(Subcommand)]
enum Commands {
    /// Sign a transaction
    Sign {
        /// Base64 encoded transaction to sign
        #[arg(long, short = 't')]
        transaction: String,
    },
    /// Sign and send a transaction
    SignAndSend {
        /// Base64 encoded transaction to sign and send
        #[arg(long, short = 't')]
        transaction: String,
    },
    /// Estimate transaction fee
    EstimateFee {
        /// Base64 encoded transaction to estimate fee for
        #[arg(long, short = 't')]
        transaction: String,
    },
    /// Sign transaction if paid
    SignIfPaid {
        /// Base64 encoded transaction to sign if paid
        #[arg(long, short = 't')]
        transaction: String,
        #[arg(long)]
        margin: Option<f64>,
    },
}

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    #[command(flatten)]
    pub args: CliArgs,
}

#[tokio::main]
async fn main() -> Result<(), KoraError> {
    let cli = Cli::parse();

    let config = load_config(&cli.args.common.config).unwrap_or_else(|e| {
        print_error(&format!("Failed to load config: {}", e));
        std::process::exit(1);
    });

    let rpc_client = get_rpc_client(&cli.args.common.rpc_url);

    if let Err(e) = config.validate(rpc_client.as_ref()).await {
        print_error(&format!("Config validation failed: {}", e));
        std::process::exit(1);
    }

    // Initialize the signer
    if !cli.args.common.skip_signer {
        let signer = init_signer_type(&cli.args.common).unwrap();
        init_signer(signer).unwrap_or_else(|e| {
            print_error(&format!("Failed to initialize signer: {}", e));
            std::process::exit(1);
        });
    }

    match cli.command {
        Some(Commands::Sign { transaction }) => {
            let rpc_client = create_rpc_client(&cli.args.common.rpc_url).await?;
            let validation = config.validation;

            let transaction = decode_b64_transaction(&transaction).map_err(|e| {
                print_error(&format!("Failed to decode transaction: {}", e));
                e
            })?;

            let (transaction, signed_tx) =
                sign_transaction(&rpc_client, &validation, transaction).await?;
            println!("Signature: {}", transaction.signatures[0]);
            println!("Signed Transaction: {}", signed_tx);
        }
        Some(Commands::SignAndSend { transaction }) => {
            if transaction.is_empty() {
                print_error("No transaction provided. Please provide a base64-encoded transaction using the --transaction flag.");
                std::process::exit(1);
            }
            let rpc_client = create_rpc_client(&cli.args.common.rpc_url).await?;
            let validation = config.validation;

            let transaction = decode_b64_transaction(&transaction).map_err(|e| {
                print_error(&format!("Failed to decode transaction: {}", e));
                e
            })?;

            let (signature, signed_tx) =
                sign_and_send_transaction(&rpc_client, &validation, transaction).await?;
            println!("Signature: {}", signature);
            println!("Signed Transaction: {}", signed_tx);
        }
        Some(Commands::EstimateFee { transaction }) => {
            let rpc_client = create_rpc_client(&cli.args.common.rpc_url).await?;

            let transaction = decode_b64_transaction(&transaction).map_err(|e| {
                print_error(&format!("Failed to decode transaction: {}", e));
                e
            })?;

            let fee = estimate_transaction_fee(&rpc_client, &transaction).await?;
            println!("Estimated fee: {} lamports", fee);
        }
        Some(Commands::SignIfPaid { transaction, margin }) => {
            let rpc_client = create_rpc_client(&cli.args.common.rpc_url).await?;
            let validation = config.validation;

            let transaction = decode_b64_transaction(&transaction).map_err(|e| {
                print_error(&format!("Failed to decode transaction: {}", e));
                e
            })?;

            let (transaction, signed_tx) =
                sign_transaction_if_paid(&rpc_client, &validation, transaction, margin).await?;

            println!("Signature: {}", transaction.signatures[0]);
            println!("Signed Transaction: {}", signed_tx);
        }
        None => {
            println!("No command specified. Use --help for usage information.");
        }
    }

    Ok(())
}

fn print_error(message: &str) {
    eprintln!("Error: {}", message);
    std::process::exit(1);
}
