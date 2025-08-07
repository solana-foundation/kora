use clap::{Parser, Subcommand};
use kora_lib::{
    args::CliArgs,
    error::KoraError,
    fee::fee::FeeConfigUtil,
    get_signer,
    rpc::{create_rpc_client, get_rpc_client},
    signer::init::init_signer_type,
    state::init_signer,
    transaction::{
        TransactionUtil, VersionedTransactionExt, VersionedTransactionResolved,
        VersionedTransactionUtilExt,
    },
    validator::config_validator::ConfigValidator,
    Config,
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

    let config = Config::load_config(&cli.args.common.config).unwrap_or_else(|e| {
        print_error(&format!("Failed to load config: {e}"));
        std::process::exit(1);
    });

    let rpc_client = get_rpc_client(&cli.args.common.rpc_url);

    if cli.args.common.validate_config || cli.args.common.validate_config_with_rpc {
        let skip_rpc_validation = !cli.args.common.validate_config_with_rpc;
        let _ = ConfigValidator::validate_with_result(
            &config,
            rpc_client.as_ref(),
            skip_rpc_validation,
        )
        .await;
        std::process::exit(0);
    } else {
        // Normal validation for non-validate-config mode
        if let Err(e) = ConfigValidator::validate(&config, rpc_client.as_ref()).await {
            print_error(&format!("Config validation failed: {e}"));
            std::process::exit(1);
        }
    }

    // Initialize the signer
    if !cli.args.common.skip_signer {
        let signer = init_signer_type(&cli.args.common).unwrap();
        init_signer(signer).unwrap_or_else(|e| {
            print_error(&format!("Failed to initialize signer: {e}"));
            std::process::exit(1);
        });
    }

    match cli.command {
        Some(Commands::Sign { transaction }) => {
            let rpc_client = create_rpc_client(&cli.args.common.rpc_url).await?;
            let validation = config.validation;

            let transaction =
                TransactionUtil::decode_b64_transaction(&transaction).map_err(|e| {
                    print_error(&format!("Failed to decode transaction: {e}"));
                    e
                })?;

            let (transaction, signed_tx) =
                transaction.sign_transaction(&rpc_client, &validation).await?;
            println!("Signature: {}", transaction.signatures[0]);
            println!("Signed Transaction: {signed_tx}");
        }
        Some(Commands::SignAndSend { transaction }) => {
            if transaction.is_empty() {
                print_error("No transaction provided. Please provide a base64-encoded transaction using the --transaction flag.");
                std::process::exit(1);
            }
            let rpc_client = create_rpc_client(&cli.args.common.rpc_url).await?;
            let validation = config.validation;

            let transaction =
                TransactionUtil::decode_b64_transaction(&transaction).map_err(|e| {
                    print_error(&format!("Failed to decode transaction: {e}"));
                    e
                })?;

            let (signature, signed_tx) =
                transaction.sign_and_send_transaction(&rpc_client, &validation).await?;
            println!("Signature: {signature}");
            println!("Signed Transaction: {signed_tx}");
        }
        Some(Commands::EstimateFee { transaction }) => {
            let rpc_client = create_rpc_client(&cli.args.common.rpc_url).await?;

            let transaction =
                TransactionUtil::decode_b64_transaction(&transaction).map_err(|e| {
                    print_error(&format!("Failed to decode transaction: {e}"));
                    e
                })?;

            // Resolve lookup tables for V0 transactions to ensure accurate fee calculation
            let mut resolved_transaction = VersionedTransactionResolved::new(&transaction);
            resolved_transaction.resolve_addresses(&rpc_client).await?;

            let fee_payer = get_signer()?;
            let fee_payer_pubkey = fee_payer.solana_pubkey();

            let fee = FeeConfigUtil::estimate_transaction_fee(
                &rpc_client,
                &resolved_transaction,
                Some(&fee_payer_pubkey),
            )
            .await?;

            println!("Estimated fee: {fee} lamports");
        }
        Some(Commands::SignIfPaid { transaction }) => {
            let rpc_client = create_rpc_client(&cli.args.common.rpc_url).await?;
            let validation = config.validation;

            let transaction =
                TransactionUtil::decode_b64_transaction(&transaction).map_err(|e| {
                    print_error(&format!("Failed to decode transaction: {e}"));
                    e
                })?;

            let mut resolved_transaction = VersionedTransactionResolved::new(&transaction);
            resolved_transaction.resolve_addresses(&rpc_client).await?;

            let (transaction, signed_tx) =
                resolved_transaction.sign_transaction_if_paid(&rpc_client, &validation).await?;

            println!("Signature: {}", transaction.signatures[0]);
            println!("Signed Transaction: {signed_tx}");
        }
        None => {
            println!("No command specified. Use --help for usage information.");
        }
    }

    Ok(())
}

fn print_error(message: &str) {
    eprintln!("Error: {message}");
    std::process::exit(1);
}
