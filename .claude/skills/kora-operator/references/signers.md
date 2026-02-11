# Kora Signers Reference (signers.toml)

The signer is the keypair that signs transactions as fee payer. A compromised signer can drain your SOL balance.

All signer types are powered by [`solana-keychain`](https://crates.io/crates/solana-keychain), a unified signer abstraction (`Signer::from_memory`, `Signer::from_turnkey`, `Signer::from_privy`, `Signer::from_vault`).

## Table of Contents

- [Pool Configuration](#pool-configuration)
- [Memory Signer (Private Key)](#memory-signer-private-key)
- [Turnkey Signer](#turnkey-signer)
- [Privy Signer](#privy-signer)
- [Vault Signer (HashiCorp)](#vault-signer-hashicorp)
- [No Signer Mode](#no-signer-mode)
- [Multi-Signer Setup](#multi-signer-setup)
- [Client-Side Signer Selection](#client-side-signer-selection)

---

## Pool Configuration

```toml
[signer_pool]
strategy = "round_robin"  # Selection strategy
```

| Strategy | Behavior |
|----------|----------|
| `round_robin` | Rotate through signers sequentially |
| `random` | Random selection each request |
| `weighted` | Weighted random based on `weight` field |

---

## Memory Signer (Private Key)

Local keypair. Fastest, simplest, but key stored on disk/env.

```toml
[[signers]]
name = "signer_1"
type = "memory"
private_key_env = "KORA_PRIVATE_KEY"
weight = 1
```

**Key formats** (auto-detected):

1. **Base58**: `KORA_PRIVATE_KEY=base58EncodedPrivateKey`
2. **U8 Array**: `KORA_PRIVATE_KEY="[123,45,67,...]"` (64 bytes)
3. **JSON file path**: `KORA_PRIVATE_KEY="/path/to/keypair.json"`

Detection order: file path -> u8 array -> base58.

---

## Turnkey Signer

Remote signing via Turnkey API. Key never leaves Turnkey infrastructure.

```toml
[[signers]]
name = "turnkey_signer"
type = "turnkey"
api_public_key_env = "TURNKEY_API_PUBLIC_KEY"
api_private_key_env = "TURNKEY_API_PRIVATE_KEY"
organization_id_env = "TURNKEY_ORG_ID"
private_key_id_env = "TURNKEY_PRIVATE_KEY_ID"
public_key_env = "TURNKEY_PUBLIC_KEY"
weight = 1
```

**Setup**: Create Turnkey account -> Create API key pair -> Create Solana wallet -> Export public key.

Required env vars:
- `TURNKEY_API_PUBLIC_KEY` - API authentication public key
- `TURNKEY_API_PRIVATE_KEY` - API authentication private key
- `TURNKEY_ORG_ID` - Organization ID
- `TURNKEY_PRIVATE_KEY_ID` - Wallet's private key ID
- `TURNKEY_PUBLIC_KEY` - Solana public key for the wallet

---

## Privy Signer

Remote signing via Privy API. Key managed by Privy.

```toml
[[signers]]
name = "privy_signer"
type = "privy"
app_id_env = "PRIVY_APP_ID"
app_secret_env = "PRIVY_APP_SECRET"
wallet_id_env = "PRIVY_WALLET_ID"
weight = 1
```

**Setup**: Create Privy app -> Create server wallet -> Note wallet ID.

Required env vars:
- `PRIVY_APP_ID` - Application ID
- `PRIVY_APP_SECRET` - Application secret
- `PRIVY_WALLET_ID` - Server wallet ID

---

## Vault Signer (HashiCorp)

Remote signing via HashiCorp Vault. Key stored in Vault's transit engine.

```toml
[[signers]]
name = "vault_signer"
type = "vault"
vault_addr_env = "VAULT_ADDR"
vault_token_env = "VAULT_TOKEN"
key_name_env = "VAULT_KEY_NAME"
pubkey_env = "VAULT_PUBKEY"
weight = 1
```

Required env vars:
- `VAULT_ADDR` - Vault server address
- `VAULT_TOKEN` - Authentication token
- `VAULT_KEY_NAME` - Key name in transit engine
- `VAULT_PUBKEY` - Solana public key

---

## No Signer Mode

Run Kora without loading any signer. Limited to read-only methods.

```bash
kora rpc start --no-load-signer
```

Available methods: `getConfig`, `getBlockhash`, `getSupportedTokens`, `estimateTransactionFee`.
Unavailable: `signTransaction`, `signAndSendTransaction`, `transferTransaction`.

---

## Multi-Signer Setup

Use multiple signers for high availability and load distribution.

```toml
[signer_pool]
strategy = "weighted"

[[signers]]
name = "primary"
type = "memory"
private_key_env = "KORA_PRIVATE_KEY_1"
weight = 3  # 3x more likely to be selected

[[signers]]
name = "secondary"
type = "turnkey"
api_public_key_env = "TURNKEY_API_PUBLIC_KEY"
api_private_key_env = "TURNKEY_API_PRIVATE_KEY"
organization_id_env = "TURNKEY_ORG_ID"
private_key_id_env = "TURNKEY_PRIVATE_KEY_ID"
public_key_env = "TURNKEY_PUBLIC_KEY"
weight = 1

[[signers]]
name = "backup"
type = "privy"
app_id_env = "PRIVY_APP_ID"
app_secret_env = "PRIVY_APP_SECRET"
wallet_id_env = "PRIVY_WALLET_ID"
weight = 1
```

All signers must be funded with SOL for fee payment. Monitor with `signer_balance_lamports` metric.

---

## Client-Side Signer Selection

Clients can request a specific signer using `signer_key` parameter on any transaction method:

```json
{
  "method": "signTransaction",
  "params": {
    "transaction": "<base64>",
    "signer_key": "<specific-signer-pubkey>"
  }
}
```

Use `getPayerSigner` or `getConfig` (which returns `fee_payers` array) to discover available signers.
