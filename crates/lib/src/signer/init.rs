use crate::{
    error::KoraError,
    rpc_server::RpcArgs,
    signer::{
        privy::types::PrivySigner, turnkey::types::TurnkeySigner, KoraSigner, SolanaMemorySigner,
        VaultSigner,
    },
};

pub fn init_signer_type(args: &RpcArgs) -> Result<KoraSigner, KoraError> {
    if args.turnkey_signer {
        init_turnkey_signer(args)
    } else if args.vault_signer {
        init_vault_signer(args)
    } else if args.privy_signer {
        init_privy_signer(args)
    } else {
        init_memory_signer(args.private_key.as_ref())
    }
}

fn init_vault_signer(config: &RpcArgs) -> Result<KoraSigner, KoraError> {
    let vault_addr = config
        .vault_addr
        .as_ref()
        .ok_or_else(|| KoraError::SigningError("Vault address required".to_string()))?;

    let vault_token = config
        .vault_token
        .as_ref()
        .ok_or_else(|| KoraError::SigningError("Vault token required".to_string()))?;

    let key_name = config
        .vault_key_name
        .as_ref()
        .ok_or_else(|| KoraError::SigningError("Vault key name required".to_string()))?;

    let pubkey = config
        .vault_pubkey
        .as_ref()
        .ok_or_else(|| KoraError::SigningError("Vault public key required".to_string()))?;

    let signer = VaultSigner::new(
        vault_addr.to_string(),
        vault_token.to_string(),
        key_name.to_string(),
        pubkey.to_string(),
    )?;

    Ok(KoraSigner::Vault(signer))
}

fn init_turnkey_signer(config: &RpcArgs) -> Result<KoraSigner, KoraError> {
    let api_pub = config
        .turnkey_api_public_key
        .as_ref()
        .ok_or_else(|| KoraError::SigningError("Turnkey API public key required".to_string()))?;
    let api_priv = config
        .turnkey_api_private_key
        .as_ref()
        .ok_or_else(|| KoraError::SigningError("Turnkey API private key required".to_string()))?;
    let api_priv_key_id = config
        .turnkey_private_key_id
        .as_ref()
        .ok_or_else(|| KoraError::SigningError("Turnkey private key ID required".to_string()))?;
    let org_id = config
        .turnkey_organization_id
        .as_ref()
        .ok_or_else(|| KoraError::SigningError("Turnkey organization ID required".to_string()))?;
    let public_key_id = config
        .turnkey_public_key
        .as_ref()
        .ok_or_else(|| KoraError::SigningError("Turnkey public key required".to_string()))?;

    let signer = TurnkeySigner::new(
        api_pub.to_string(),
        api_priv.to_string(),
        org_id.to_string(),
        api_priv_key_id.to_string(),
        public_key_id.to_string(),
    )?;

    Ok(KoraSigner::Turnkey(signer))
}

fn init_privy_signer(config: &RpcArgs) -> Result<KoraSigner, KoraError> {
    let app_id = config
        .privy_app_id
        .clone()
        .or_else(|| std::env::var("PRIVY_APP_ID").ok())
        .ok_or_else(|| KoraError::SigningError("Privy app ID required".to_string()))?;

    let app_secret = config
        .privy_app_secret
        .clone()
        .or_else(|| std::env::var("PRIVY_APP_SECRET").ok())
        .ok_or_else(|| KoraError::SigningError("Privy app secret required".to_string()))?;

    let wallet_id = config
        .privy_wallet_id
        .clone()
        .or_else(|| std::env::var("PRIVY_WALLET_ID").ok())
        .ok_or_else(|| KoraError::SigningError("Privy wallet ID required".to_string()))?;

    let privy_signer = PrivySigner::new(app_id, app_secret, wallet_id);

    Ok(KoraSigner::Privy(privy_signer))
}

fn init_memory_signer(private_key: Option<&String>) -> Result<KoraSigner, KoraError> {
    let key = private_key.ok_or_else(|| {
        KoraError::SigningError("Private key required for memory signer".to_string())
    })?;

    let signer = SolanaMemorySigner::from_private_key_string(key)?;
    Ok(KoraSigner::Memory(signer))
}
