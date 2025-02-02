use clap::{Parser, Subcommand};
use kora_lib::{
    args::Args, config::load_config, error::KoraError, log::LoggingFormat, rpc::{create_rpc_client, get_rpc_client}, signer::init::init_signer_type, state::init_signer, transaction::{
        decode_b58_transaction, estimate_transaction_fee,
        sign_and_send_transaction, sign_transaction, sign_transaction_if_paid, TokenPriceInfo,
    },
};
use std::io::{self, Read};

#[derive(Subcommand)]
enum Commands {
    /// Sign a transaction
    Sign {
        #[arg(long)]
        rpc_url: String,
    },
    /// Sign and send a transaction
    SignAndSend {
        #[arg(long)]
        rpc_url: String,
    },
    /// Estimate transaction fee
    EstimateFee {
        #[arg(long)]
        rpc_url: String,
    },
    /// Sign transaction if paid
    SignIfPaid {
        #[arg(long)]
        rpc_url: String,
        #[arg(long)]
        margin: Option<f64>,
        #[arg(long)]
        token_price: Option<f64>,
    },
}

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    /// Logging format (standard or json)
    #[arg(long)]
    logging_format: Option<LoggingFormat>,

    #[command(flatten)]
    pub args: Args,
}

#[tokio::main]
async fn main() -> Result<(), KoraError> {
    let cli = Cli::parse();

    let config = load_config(&cli.args.config).unwrap_or_else(|e| {
        std::process::exit(1);
    });

    let rpc_client = get_rpc_client(&cli.args.rpc_url);

    if let Err(e) = config.validate(rpc_client.as_ref()).await {
        std::process::exit(1);
    }

    // Initialize the signer
    let signer = init_signer_type(&cli.args).unwrap();
    init_signer(signer).unwrap_or_else(|e| {
        std::process::exit(1);
    });

    match cli.command {
        Some(Commands::Sign { rpc_url }) => {
            let rpc_client = create_rpc_client(&rpc_url).await?;
            let validation = config.validation;

            // Read transaction from stdin
            let mut input = String::new();
            io::stdin().read_to_string(&mut input)?;
            let transaction = decode_b58_transaction(input.trim())?;

            let (transaction, signed_tx) = sign_transaction(&rpc_client, &validation, transaction).await?;
            println!("Signature: {}", transaction.signatures[0]);
            println!("Signed Transaction: {}", signed_tx);
        }
        Some(Commands::SignAndSend { rpc_url }) => {
            let rpc_client = create_rpc_client(&rpc_url).await?;
            let validation = config.validation;

            // Read transaction from stdin
            let mut input = String::new();
            io::stdin().read_to_string(&mut input)?;
            let transaction = decode_b58_transaction(input.trim())?;

            let (signature, signed_tx) = sign_and_send_transaction(&rpc_client, &validation, transaction).await?;
            println!("Signature: {}", signature);
            println!("Signed Transaction: {}", signed_tx);
        }
        Some(Commands::EstimateFee { rpc_url }) => {
            let rpc_client = create_rpc_client(&rpc_url).await?;

            // Read transaction from stdin
            let mut input = String::new();
            io::stdin().read_to_string(&mut input)?;
            let transaction = decode_b58_transaction(input.trim())?;

            let fee = estimate_transaction_fee(&rpc_client, &transaction).await?;
            println!("Estimated fee: {} lamports", fee);
        }
        Some(Commands::SignIfPaid { rpc_url, margin, token_price }) => {
            let rpc_client = create_rpc_client(&rpc_url).await?;
            let validation = config.validation;

            // Read transaction from stdin
            let mut input = String::new();
            io::stdin().read_to_string(&mut input)?;
            let transaction = decode_b58_transaction(input.trim())?;

            let token_price_info = token_price.map(|price| TokenPriceInfo { price });

            let (transaction, signed_tx) = sign_transaction_if_paid(
                &rpc_client,
                &validation,
                transaction,
                margin,
                token_price_info,
            )
            .await?;

            println!("Signature: {}", transaction.signatures[0]);
            println!("Signed Transaction: {}", signed_tx);
        }
        None => {
            println!("No command specified. Use --help for usage information.");
        }
    }

    Ok(())
}
