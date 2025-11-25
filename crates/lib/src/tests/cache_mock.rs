use crate::{config::Config, error::KoraError};
use mockall::mock;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{account::Account, pubkey::Pubkey};

mock! {
    pub CacheUtil {
        pub async fn init() -> Result<(), KoraError>;
        pub async fn get_account(
            config: &Config,
            rpc_client: &RpcClient,
            pubkey: &Pubkey,
            force_refresh: bool,
        ) -> Result<Account, KoraError>;
    }
}
