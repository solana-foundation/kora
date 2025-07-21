use super::KoraError;
use deadpool_redis::{Config, Pool, Runtime};
use redis::AsyncCommands;
use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;

const TOKEN_ACCOUNT_CACHE_TTL: u64 = 3600 * 24; // 1 day in seconds

#[derive(Clone)]
pub struct TokenAccountCache {
    pool: Pool,
}

impl TokenAccountCache {
    pub fn new(redis_url: &str) -> Result<Self, KoraError> {
        let cfg = Config::from_url(redis_url);
        let pool = cfg.create_pool(Some(Runtime::Tokio1)).map_err(|e| {
            KoraError::InternalServerError(format!("Failed to create Redis pool: {e}"))
        })?;

        Ok(Self { pool })
    }

    pub async fn get_token_account(
        &self,
        user: &Pubkey,
        mint: &Pubkey,
    ) -> Result<Option<Pubkey>, KoraError> {
        let key = format!("token_account:{user}:{mint}");
        let mut conn = self.pool.get().await.map_err(|e| {
            KoraError::InternalServerError(format!("Failed to get Redis connection: {e}"))
        })?;

        let result: Option<String> = conn.get(&key).await.map_err(|e| {
            KoraError::InternalServerError(format!("Failed to get from Redis: {e}"))
        })?;

        match result {
            Some(pubkey_str) => {
                let pubkey = Pubkey::from_str(&pubkey_str).map_err(|e| {
                    KoraError::InternalServerError(format!("Invalid pubkey in cache: {e}"))
                })?;
                Ok(Some(pubkey))
            }
            None => Ok(None),
        }
    }

    pub async fn set_token_account(
        &self,
        user: &Pubkey,
        mint: &Pubkey,
        token_account: &Pubkey,
    ) -> Result<(), KoraError> {
        let key = format!("token_account:{user}:{mint}");
        let mut conn = self.pool.get().await.map_err(|e| {
            KoraError::InternalServerError(format!("Failed to get Redis connection: {e}"))
        })?;

        conn.set_ex::<_, _, ()>(&key, token_account.to_string(), TOKEN_ACCOUNT_CACHE_TTL)
            .await
            .map_err(|e| KoraError::InternalServerError(format!("Failed to set in Redis: {e}")))?;

        Ok(())
    }

    pub async fn _invalidate_token_account(
        &self,
        user: &Pubkey,
        mint: &Pubkey,
    ) -> Result<(), KoraError> {
        let key = format!("token_account:{user}:{mint}");
        let mut conn = self.pool.get().await.map_err(|e| {
            KoraError::InternalServerError(format!("Failed to get Redis connection: {e}"))
        })?;

        conn.del::<_, ()>(&key).await.map_err(|e| {
            KoraError::InternalServerError(format!("Failed to delete from Redis: {e}"))
        })?;

        Ok(())
    }
}
