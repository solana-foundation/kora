use std::{str::FromStr, sync::Arc, time::Duration};

use anyhow::{anyhow, bail, Result};
use base64::{engine::general_purpose::STANDARD as B64, Engine};
use reqwest::Client;
use serde_json::{json, Value};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_commitment_config::CommitmentConfig;
use solana_loader_v3_interface::{instruction as v3, state::UpgradeableLoaderState};
use solana_sdk::{
    hash::Hash,
    instruction::Instruction,
    message::Message,
    pubkey::Pubkey,
    signature::{Keypair, Signature},
    signer::Signer,
    transaction::Transaction,
};

const WRITE_CHUNK_SIZE: usize = 900;
pub const BPF_LOADER_UPGRADEABLE: Pubkey =
    solana_sdk::pubkey!("BPFLoaderUpgradeab1e11111111111111111111111");

/// Outcome of a `signTransaction` probe. Kora simulates before it validates, so `SimFailed` means
/// the policy was never reached (the transaction failed on-chain simulation first).
pub enum Probe {
    Signed,
    PolicyReject(String),
    SimFailed(String),
}

/// A thin client over a live Kora paymaster plus the Solana RPC, used to provision real on-chain
/// accounts and to probe the paymaster with crafted transactions.
pub struct Harness {
    http: Client,
    rpc: Arc<RpcClient>,
    kora_url: String,
    payer: Pubkey,
}

impl Harness {
    pub async fn connect(kora_url: &str, rpc_url: &str) -> Result<Self> {
        let http = Client::builder().timeout(Duration::from_secs(60)).build()?;
        let rpc = Arc::new(RpcClient::new_with_commitment(
            rpc_url.to_string(),
            CommitmentConfig::confirmed(),
        ));
        let payer = fetch_payer(&http, kora_url).await?;
        Ok(Self { http, rpc, kora_url: kora_url.to_string(), payer })
    }

    pub fn payer(&self) -> Pubkey {
        self.payer
    }

    pub fn rpc(&self) -> &RpcClient {
        &self.rpc
    }

    pub async fn balance(&self) -> Result<u64> {
        Ok(self.rpc.get_balance(&self.payer).await?)
    }

    pub async fn probe(&self, ixs: &[Instruction], signers: &[&Keypair]) -> Result<Probe> {
        let resp = self.rpc_call("signTransaction", ixs, signers).await?;
        if let Some(err) = resp.get("error") {
            let msg = err.to_string();
            let lower = msg.to_ascii_lowercase();
            if lower.contains("simulation failed") || lower.contains("failed to simulate") {
                return Ok(Probe::SimFailed(msg));
            }
            return Ok(Probe::PolicyReject(msg));
        }
        if resp["result"]["signed_transaction"].is_string() {
            return Ok(Probe::Signed);
        }
        bail!("unexpected signTransaction response: {resp}")
    }

    pub async fn send(&self, ixs: &[Instruction], signers: &[&Keypair]) -> Result<Signature> {
        let resp = self.rpc_call("signAndSendTransaction", ixs, signers).await?;
        if let Some(err) = resp.get("error") {
            bail!("Kora rejected: {err}");
        }
        let sig = Signature::from_str(
            resp["result"]["signature"].as_str().ok_or_else(|| anyhow!("no signature: {resp}"))?,
        )?;
        for _ in 0..120 {
            if let Some(status) = self
                .rpc
                .get_signature_status_with_commitment(&sig, CommitmentConfig::confirmed())
                .await?
            {
                return match status {
                    Ok(()) => Ok(sig),
                    Err(e) => bail!("transaction {sig} failed on-chain: {e}"),
                };
            }
            tokio::time::sleep(Duration::from_millis(500)).await;
        }
        bail!("timed out confirming {sig}")
    }

    pub async fn create_buffer(&self) -> Result<Keypair> {
        let buffer = Keypair::new();
        let lamports = self
            .rpc
            .get_minimum_balance_for_rent_exemption(UpgradeableLoaderState::size_of_buffer(0))
            .await?;
        let ixs = v3::create_buffer(&self.payer, &buffer.pubkey(), &self.payer, lamports, 0)?;
        self.send(&ixs, &[&buffer]).await?;
        Ok(buffer)
    }

    pub async fn create_buffer_with_program(&self, bytes: &[u8]) -> Result<Keypair> {
        let buffer = Keypair::new();
        let lamports = self
            .rpc
            .get_minimum_balance_for_rent_exemption(UpgradeableLoaderState::size_of_buffer(
                bytes.len(),
            ))
            .await?;
        let create =
            v3::create_buffer(&self.payer, &buffer.pubkey(), &self.payer, lamports, bytes.len())?;
        self.send(&create, &[&buffer]).await?;
        for (i, chunk) in bytes.chunks(WRITE_CHUNK_SIZE).enumerate() {
            let ix = v3::write(
                &buffer.pubkey(),
                &self.payer,
                (i * WRITE_CHUNK_SIZE) as u32,
                chunk.to_vec(),
            );
            self.send(&[ix], &[]).await?;
        }
        Ok(buffer)
    }

    pub async fn deploy_program(&self, bytes: &[u8]) -> Result<(Pubkey, Pubkey)> {
        let buffer = self.create_buffer_with_program(bytes).await?;
        let program = Keypair::new();
        let program_data =
            Pubkey::find_program_address(&[program.pubkey().as_ref()], &BPF_LOADER_UPGRADEABLE).0;
        let program_lamports = self
            .rpc
            .get_minimum_balance_for_rent_exemption(UpgradeableLoaderState::size_of_program())
            .await?;
        let ixs = v3::deploy_with_max_program_len(
            &self.payer,
            &program.pubkey(),
            &buffer.pubkey(),
            &self.payer,
            program_lamports,
            bytes.len(),
        )?;
        self.send(&ixs, &[&program]).await?;
        Ok((program.pubkey(), program_data))
    }

    pub async fn close_account(&self, close_addr: &Pubkey, program: Option<&Pubkey>) -> Result<()> {
        let ix = v3::close_any(close_addr, &self.payer, Some(&self.payer), program);
        self.send(&[ix], &[]).await?;
        Ok(())
    }

    async fn rpc_call(
        &self,
        method: &str,
        ixs: &[Instruction],
        signers: &[&Keypair],
    ) -> Result<Value> {
        let mut last = String::new();
        for attempt in 0..6 {
            let blockhash = self.rpc.get_latest_blockhash().await?;
            let tx_b64 = build_b64(&self.payer, blockhash, ixs, signers)?;
            let user_id = format!("suite-{}", Pubkey::new_unique());
            let sent = self
                .http
                .post(&self.kora_url)
                .json(&json!({
                    "jsonrpc": "2.0", "id": 1, "method": method,
                    "params": {"transaction": tx_b64, "user_id": user_id}
                }))
                .send()
                .await;
            match sent {
                Ok(r) => match r.error_for_status() {
                    Ok(ok) => {
                        let resp: Value = ok.json().await?;
                        let stale = resp
                            .get("error")
                            .and_then(|e| e.get("message"))
                            .and_then(|m| m.as_str())
                            .is_some_and(|m| m.contains("Blockhash not found"));
                        if stale {
                            last = "stale blockhash".into();
                        } else {
                            return Ok(resp);
                        }
                    }
                    Err(e) => last = e.to_string(),
                },
                Err(e) => last = e.to_string(),
            }
            tokio::time::sleep(Duration::from_millis(400 * (attempt + 1))).await;
        }
        bail!("rpc_call {method} failed after retries: {last}")
    }
}

fn build_b64(
    fee_payer: &Pubkey,
    blockhash: Hash,
    ixs: &[Instruction],
    signers: &[&Keypair],
) -> Result<String> {
    let msg = Message::new_with_blockhash(ixs, Some(fee_payer), &blockhash);
    let mut tx = Transaction::new_unsigned(msg);
    if !signers.is_empty() {
        tx.partial_sign(signers, blockhash);
    }
    Ok(B64.encode(bincode::serialize(&tx)?))
}

async fn fetch_payer(http: &Client, url: &str) -> Result<Pubkey> {
    let resp: Value = http
        .post(url)
        .json(&json!({"jsonrpc":"2.0","id":1,"method":"getPayerSigner","params":[]}))
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    Pubkey::from_str(
        resp["result"]["signer_address"]
            .as_str()
            .ok_or_else(|| anyhow!("getPayerSigner missing signer_address: {resp}"))?,
    )
    .map_err(|e| anyhow!("parsing kora pubkey: {e}"))
}
