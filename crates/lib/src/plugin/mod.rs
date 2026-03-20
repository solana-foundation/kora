use async_trait::async_trait;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::pubkey::Pubkey;
use std::collections::HashSet;

use crate::{
    config::{Config, TransactionPluginType},
    error::KoraError,
    transaction::VersionedTransactionResolved,
};

mod plugin_gas_swap;

use plugin_gas_swap::GasSwapPlugin;

#[derive(Debug, Clone, Copy)]
pub enum PluginExecutionContext {
    SignTransaction,
    SignAndSendTransaction,
    SignBundle,
    SignAndSendBundle,
}

impl PluginExecutionContext {
    pub(super) fn method_name(self) -> &'static str {
        match self {
            Self::SignTransaction => "signTransaction",
            Self::SignAndSendTransaction => "signAndSendTransaction",
            Self::SignBundle => "signBundle",
            Self::SignAndSendBundle => "signAndSendBundle",
        }
    }
}

#[async_trait]
trait TransactionPlugin: Send + Sync {
    async fn validate(
        &self,
        transaction: &mut VersionedTransactionResolved,
        _config: &Config,
        _rpc_client: &RpcClient,
        fee_payer: &Pubkey,
        context: PluginExecutionContext,
    ) -> Result<(), KoraError>;
}

pub struct TransactionPluginRunner {
    plugins: Vec<Box<dyn TransactionPlugin>>,
}

impl TransactionPluginRunner {
    pub fn from_config(config: &Config) -> Self {
        let mut enabled = HashSet::new();
        let mut plugins: Vec<Box<dyn TransactionPlugin>> = Vec::new();

        // TODO: WasmPlugin — operators should be able to register custom plugins via a config
        // path (e.g. `plugins = [{type = "wasm", path = "my_plugin.wasm"}]`) without requiring a
        // Kora source change or new release. A WasmPlugin implementing TransactionPlugin would
        // load a .wasm module at startup and call it for each transaction. The migration is clean:
        // WasmPlugin sits alongside typed built-ins until we're ready to drop hardcoded dispatch.
        //
        //   pub struct WasmPlugin { engine: wasmtime::Engine, module: wasmtime::Module }
        //   impl TransactionPlugin for WasmPlugin { ... }
        //
        // TransactionPluginType would gain a `Wasm { path: PathBuf }` variant alongside GasSwap.
        for plugin in &config.kora.plugins.enabled {
            if !enabled.insert(plugin.clone()) {
                continue;
            }

            match plugin {
                TransactionPluginType::GasSwap => {
                    plugins.push(Box::new(GasSwapPlugin));
                }
            }
        }

        Self { plugins }
    }

    pub async fn run(
        &self,
        transaction: &mut VersionedTransactionResolved,
        config: &Config,
        rpc_client: &RpcClient,
        fee_payer: &Pubkey,
        context: PluginExecutionContext,
    ) -> Result<(), KoraError> {
        for plugin in &self.plugins {
            plugin.validate(transaction, config, rpc_client, fee_payer, context).await?;
        }

        Ok(())
    }
}
