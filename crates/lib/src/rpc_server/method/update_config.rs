#[cfg(feature = "tests")]
use crate::{config::Config, state};
#[cfg(feature = "tests")]
use jsonrpsee::core::RpcResult;
#[cfg(feature = "tests")]
use serde_json::Value;

/// Update the server configuration (dev/test only)
/// This allows hot-reloading the config without restarting the server
#[cfg(feature = "tests")]
pub async fn update_config(new_config: Config) -> RpcResult<Value> {
    log::info!("Updating server configuration (dev mode)");

    // Update the global config
    if let Err(e) = state::update_config(new_config) {
        log::error!("Failed to update config: {}", e);
        return Err(jsonrpsee::core::Error::Custom(format!("Failed to update config: {}", e)));
    }

    log::info!("✓ Configuration updated successfully");
    Ok(serde_json::json!({
        "status": "success",
        "message": "Configuration updated successfully"
    }))
}
