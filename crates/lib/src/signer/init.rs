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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        log::LoggingFormat,
        rpc_server::args::{AuthArgs, RpcArgs},
        tests::config_mock::ConfigMockBuilder,
    };
    use std::path::PathBuf;

    #[tokio::test]
    async fn test_init_signers_skip_signer() {
        let _m = ConfigMockBuilder::new().build_and_setup();

        let args = RpcArgs {
            port: 8080,
            logging_format: LoggingFormat::Standard,
            signers_config: None,
            skip_signer: true,
            auth_args: AuthArgs { api_key: None, hmac_secret: None },
        };

        let result = init_signers(&args).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_init_signers_no_config_no_skip() {
        let _m = ConfigMockBuilder::new().build_and_setup();

        let args = RpcArgs {
            port: 8080,
            logging_format: LoggingFormat::Standard,
            signers_config: None,
            skip_signer: false,
            auth_args: AuthArgs { api_key: None, hmac_secret: None },
        };

        let result = init_signers(&args).await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), KoraError::ValidationError(_)));
    }

    #[tokio::test]
    async fn test_init_signers_with_invalid_config_path() {
        let _m = ConfigMockBuilder::new().build_and_setup();

        let args = RpcArgs {
            port: 8080,
            logging_format: LoggingFormat::Standard,
            signers_config: Some(PathBuf::from("/nonexistent/config.toml")),
            skip_signer: false,
            auth_args: AuthArgs { api_key: None, hmac_secret: None },
        };

        let result = init_signers(&args).await;
        assert!(result.is_err());
    }
}
