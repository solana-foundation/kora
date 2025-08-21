use crate::{
    error::KoraError,
    rpc_server::RpcArgs,
    signer::{SignerPool, SignerPoolConfig},
    state::init_signer_pool,
};

/// Initialize signer(s) based on RPC args - supports multi-signer mode or skip signers
pub async fn init_signers(args: &RpcArgs) -> Result<(), KoraError> {
    if args.skip_signer {
        log::info!("Skipping signer initialization as requested");
        return Ok(());
    }

    if let Some(config_path) = &args.signers_config {
        // Multi-signer mode: load and initialize signer pool
        log::info!("Initializing multi-signer mode from config: {}", config_path.display());

        let config = SignerPoolConfig::load_config(config_path)?;
        let pool = SignerPool::from_config(config).await?;

        init_signer_pool(pool)?;
        log::info!("Multi-signer pool initialized successfully");
    } else {
        return Err(KoraError::ValidationError(
            "Signers configuration is required unless using --no-load-signer".to_string(),
        ));
    }

    Ok(())
}
