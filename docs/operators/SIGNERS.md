# Kora Signers Guide

*Last Updated: 2025-08-22*

## What is a Signer?

A **signer** is the cryptographic keypair that your Kora node uses to sign Solana transactions as the fee payer. When users send transactions to your Kora node, it validates them and co-signs them with your signer's private key to cover the SOL transaction fees.

**Note**: By default, token payment fees are sent to the signer's address. However, you can configure a separate `payment_address` in your `kora.toml` to receive payments at a different address while keeping your signer separate. See [Configuration Guide](./CONFIGURATION.md) for details.

Your signer keypair has direct access to your SOL funds used for paying transaction fees. If compromised, an attacker could:
- Drain your SOL balance
- Sign unauthorized transactions
- Disrupt your paymaster service

## Signer Configuration

The Kora RPC CLI requires a `signer.toml` to be specified via the `--signers-config` flag. The `singer.toml` file allows you to configure the signer(s) and signer configuration for your node. `signer.toml` has two sections:

1. `[signer_pool]` - Configuration for the signer pool
2. `[[signers]]` - Configuration for each signer (at least one signer is required unless using `--no-load-signer` flag which has limited functionality)

#### `[signer_pool]`
The signer pool configuration specifies attributes specific to the signer pool as a whole:

- `strategy` - The selection strategy for choosing signers. Available strategies are:
  - `round_robin` (default)- Cycle through signers in order.
  - `random` - Select signers randomly.
  - `weighted` - Select signers based on weight.

#### `[[signers]]`

Each signer is configured with:
-  a `name`: a human-readable identifier for the signer and must be unique within the signer pool
- an optional `weight`: a number that specifies the weight of the signer if `strategy` is `weighted`
- a `type` and type-specific configuration (see [Signer Types](#signer-types))

One signer is required unless using the `--no-load-signer` flag which has limited functionality. For production deployments, it is recommended to configure multiple signers for improved reliability and performance.

### Example

Here's an example `signers.toml` file to defines a round-robin signer pool with three signers (*note: we'll cover the different signer types/configurations in the next section*):

```toml
[signer_pool]
# Selection strategy: round_robin, random, weighted
strategy = "round_robin"

# Primary memory signer
[[signers]]
name = "signer_1"
type = "memory"
private_key_env = "SIGNER_1_PRIVATE_KEY"
# weight = 1 # Not required if strategy is not weighted

# Backup memory signer
[[signers]]
name = "signer_2"
type = "memory"
private_key_env = "SIGNER_2_PRIVATE_KEY"
# weight = 1 # Not required if strategy is not weighted

# Turnkey signer for high-value operations
[[signers]]
name = "signer_3_turnkey"
type = "turnkey"
api_public_key_env = "TURNKEY_API_PUBLIC_KEY"
api_private_key_env = "TURNKEY_API_PRIVATE_KEY"
organization_id_env = "TURNKEY_ORG_ID"
private_key_id_env = "TURNKEY_PRIVATE_KEY_ID"
public_key_env = "TURNKEY_PUBLIC_KEY"
# weight = 2  # Higher weight = selected more often
```


### Environment Variables

Set environment variables for all configured signers:

```bash
# Memory signers
SIGNER_1_PRIVATE_KEY="your_base58_private_key_1"
SIGNER_2_PRIVATE_KEY="your_base58_private_key_2"

# Turnkey signer
TURNKEY_API_PUBLIC_KEY="your_turnkey_api_public_key"
TURNKEY_API_PRIVATE_KEY="your_turnkey_api_private_key"
TURNKEY_ORG_ID="your_turnkey_organization_id"
TURNKEY_PRIVATE_KEY_ID="your_turnkey_private_key_id"
TURNKEY_PUBLIC_KEY="your_turnkey_public_key"
```

### Start Kora with Signers Configuration

```bash
kora --config path/to/kora.toml rpc start --signers-config path/to/signers.toml
```


## Signer Types

Kora supports four main signer types, each with different security and operational characteristics (and a no-signer option for limited testing):

- [Private Key](#private-key-signer) - simple, self-managed
- [Turnkey](#turnkey-signer) - key management service
- [Privy](#privy-signer) - key management service
- Vault - HashiCorp Vault integration
- [No Signer](#no-signer) - no signer (for limited testing)

## Private Key Signer

The simplest approach - store your private key directly in environment variables or pass via CLI flags. Kora accepts private keys in three formats:

#### 1. Base58 Format (Default)
Standard Solana base58 encoded private key:

```bash
KORA_PRIVATE_KEY="5KKsLVU6TcbVDK4BS6K1DGDxnh4Q9xjYJ8XaDCG5t8ht..."
```

#### 2. U8Array Format
Array of 64 bytes in JSON format:

```bash
KORA_PRIVATE_KEY="[174, 47, 154, 16, 202, 193, 206, 113, 199, 190, 53, 133, 169, 175, 31, 56, 222, 53, 138, 189, 224, 216, 117, 173, 10, 149, 53, 45, 73, 251, 237, 246, 15, 185, 186, 82, 177, 240, 148, 69, 241, 227, 167, 80, 141, 89, 240, 121, 121, 35, 172, 247, 68, 251, 226, 218, 48, 63, 176, 109, 168, 89, 238, 135]"
```

#### 3. JSON File Path
Path to a JSON file containing the keypair:

```bash
KORA_PRIVATE_KEY="/path/to/keypair.json"
```

Where `keypair.json` contains:
```json
[174, 47, 154, 16, 202, 193, 206, 113, 199, 190, 53, 133, 169, 175, 31, 56, 222, 53, 138, 189, 224, 216, 117, 173, 10, 149, 53, 45, 73, 251, 237, 246, 15, 185, 186, 82, 177, 240, 148, 69, 241, 227, 167, 80, 141, 89, 240, 121, 121, 35, 172, 247, 68, 251, 226, 218, 48, 63, 176, 109, 168, 89, 238, 135]
```

### Generate New Keypair

You can create a new keypair for your Kora node using the Solana CLI:

```bash
# Generate new keypair file
solana-keygen new --outfile ~/.config/solana/kora-keypair.json

# Get the public key
solana-keygen pubkey ~/.config/solana/kora-keypair.json

# Fund with SOL for transaction fees
solana transfer --from <your-funding-wallet> <kora-public-key> 0.1
```

### Signer.toml Configuration

Required variables:

- `name` - The name of the signer
- `type` - The type of signer (must be `memory`)
- `private_key_env` - The environment variable containing the private key

```toml
[[signers]]
name = "my_memory_signer"
type = "memory"
private_key_env = "KORA_PRIVATE_KEY" # (or your environment variable name)
```


## Turnkey Signer

[Turnkey](https://www.turnkey.com/) provides enterprise-grade key management with hardware security modules (HSMs) and policy controls.

### Prerequisites

You will need a **Turnkey Account** to use the Turnkey signer. Sign up at [turnkey.com](https://www.turnkey.com/)

### Setup

You will need five keys to use the Turnkey signer:

- Turnkey organization ID
- Turnkey API public key
- Turnkey API private key
- Turnkey private key ID
- Turnkey public key

Let's fetch them from Turnkey:

#### 1. Turnkey Organization

Click the user menu in the top right corner of the Turnkey dashboard and copy the organization ID: 

![Turnkey Organization ID](../assets/img/signers/turnkey-org.jpg)

Store the organization ID in an environment variable:
```bash
TURNKEY_ORGANIZATION_ID="your_organization_id"
```

#### 2. Turnkey API Keys

- Click the user menu in the top right corner of the Turnkey dashboard and click "Account Settings".
- Under "API Keys", click "+ Create API Key".
- Select "Generate API keys in-browser"
- Enter a name for the API key and click "Continue"
- Save the public and private keys and click "Approve"

![Turnkey API Keys](../assets/img/signers/turnkey-api.jpg)

Store the API public and private keys in environment variables:
```bash
TURNKEY_API_PUBLIC_KEY="your_turnkey_api_public_key"
TURNKEY_API_PRIVATE_KEY="your_turnkey_api_private_key"
```

#### 3. Turnkey Wallet Keys

From the main menu, navigate to ["Wallets"](https://app.turnkey.com/dashboard/wallets) and click "Create Private Key".

We are going to create a new ED25519 private key with "Solana" asset address type:

![Turnkey Wallets](../assets/img/signers/turnkey-pk.jpg)

Click "Continue" and then "Approve".

From your wallets page, you should see your new private key. Click on it to view the details. You will need to copy the "Private key ID" and wallet "Address". Save them to environment variables:

```bash
TURNKEY_PRIVATE_KEY_ID="your_private_key_id" #7936...
TURNKEY_PUBLIC_KEY="your_solana_address" # 4gBe...
```

![Turnkey Wallet Details](../assets/img/signers/turnkey-pk2.jpg)

You will need to fund the wallet with SOL to pay for transaction fees.

### Configure Environment Variables

You should now have the following environment variables:

```bash
# .env file
TURNKEY_ORGANIZATION_ID="your_organization_id"
TURNKEY_API_PUBLIC_KEY="your_turnkey_api_public_key"
TURNKEY_API_PRIVATE_KEY="your_turnkey_api_private_key"
TURNKEY_PRIVATE_KEY_ID="your_private_key_id"
TURNKEY_PUBLIC_KEY="your_solana_public_key"
```

See [.env.example](../../.env.example) for a complete example.

For support with Turnkey, see the [Turnkey documentation](https://docs.turnkey.com/embedded-wallets/overview).

### Signer.toml Configuration

Required variables:

- `name` - The name of the signer
- `type` - The type of signer (must be `turnkey`)
- `api_public_key_env` - The environment variable containing the Turnkey API public key
- `api_private_key_env` - The environment variable containing the Turnkey API private key
- `organization_id_env` - The environment variable containing the Turnkey organization ID
- `private_key_id_env` - The environment variable containing the Turnkey private key ID
- `public_key_env` - The environment variable containing the Turnkey public key

```toml
[[signers]]
name = "my_turnkey_signer"
type = "turnkey"
api_public_key_env = "TURNKEY_API_PUBLIC_KEY"
api_private_key_env = "TURNKEY_API_PRIVATE_KEY"
organization_id_env = "TURNKEY_ORG_ID"
private_key_id_env = "TURNKEY_PRIVATE_KEY_ID"
public_key_env = "TURNKEY_PUBLIC_KEY"
```

## Privy Signer

[Privy](https://www.privy.io/) offers embedded wallet infrastructure with secure key management for Web3 applications.

### Prerequisites

You will need a **Privy Account** to use the Privy signer. Sign up at [privy.io](https://www.privy.io/)

### Setup 

You will need three keys to use the Privy signer:

- Privy App ID
- Privy App Secret
- Privy Wallet ID

Let's fetch them from Privy:

#### 1. Privy App ID

From your dashboard, select the application you want to use for Kora (or click "+ New app" if you don't have one).

Select "Retrieve API Keys" anc click "+ New Secret":

![Privy Wallets](../assets/img/signers/privy-app.jpg)

Copy your "App ID" and "App Secret" and store them in environment variables:

```bash
PRIVY_APP_ID="your_privy_app_id"
PRIVY_APP_SECRET="your_privy_app_secret"
```

#### 2. Privy Wallet

Next, we'll need to create a new wallet for Kora. From your dashboard's sidebar click "Wallets" under "Wallet Infrastructure", and select "New Wallet".

Select "Solana" as the blockchain and click "Save".

Click on the wallet to view the details:

![Privy Wallets](../assets/img/signers/privy-wallet.jpg)

Copy the "Wallet ID" and store them in environment variables:

```bash
PRIVY_WALLET_ID="your_privy_wallet_id"
```

You will need to fund the wallet with SOL to pay for transaction fees.

### Configure Environment Variables

You should now have the following environment variables:

```bash
# .env file
PRIVY_APP_ID="your_privy_app_id"
PRIVY_APP_SECRET="your_privy_app_secret"
PRIVY_WALLET_ID="your_wallet_id"
```

See [.env.example](../../.env.example) for a complete example.


For support with Privy, see the [Privy documentation](https://docs.privy.io/wallets/overview).

### Signer.toml Configuration

Required variables:

- `name` - The name of the signer
- `type` - The type of signer (must be `privy`)
- `app_id_env` - The environment variable containing the Privy app ID
- `app_secret_env` - The environment variable containing the Privy app secret
- `wallet_id_env` - The environment variable containing the Privy wallet ID

```toml
[[signers]]
name = "my_privy_signer"
type = "privy"
app_id_env = "PRIVY_APP_ID"
app_secret_env = "PRIVY_APP_SECRET"
wallet_id_env = "PRIVY_WALLET_ID"
```

## No Signer

If no signer is configured, Kora will throw an error. If you want to run Kora without a signer, you run it with the `--no-signer` flag:

```bash
kora --config path/to/kora.toml rpc start --no-signer
```

Note that this will limit your node to only processing requests that do not require a signer.

## Troubleshooting

### Quick Reference

| Error Message | Signer Type | Quick Fix |
| ---- | ---- | ---- |
| "At least one signer must be configured" | Any | Add at least one signer to config |
| "Failed to read config file" | Any | Check file path and contents |
| "Failed to parse signers config TOML" | Any | Check file format and signer contents |
| "Duplicate signer name" | Any | Ensure each signer is uniquely named in the configuration |
| "Invalid base58 string" | Private Key | Check key format, no extra spaces |
| "Invalid private key length" | Private Key | Use complete 64-byte Solana key |
| "Turnkey {key} required" | Turnkey | Set `TURNKEY_{key}` |
| "Privy {key} required" | Privy | Set `PRIVY_{key}` |
| "Vault {key} required" | Vault | Set `VAULT_{key}` |
| "Failed to create Vault client" | Vault | Verify Vault credentials|
| "Failed to sign with \[service\]" | Any | Check service status & credentials & rate limits |
| "Signer pool not initialized" | Multi-Signer | Check `signers.toml` path and format |
| "Cannot create empty signer pool" | Multi-Signer | Add at least one signer to config |
| "Signer with pubkey ... not found" | Multi-Signer | Check signer hint matches configured signers |
| "Signers configuration is required unless using --no-load-signer" | Any | Add a signers configuration file |

### General Debugging Tips

#### Enable Verbose Logging
Add detailed logging to diagnose issues:
```bash
RUST_LOG=debug kora rpc --with-turnkey-signer
```

## Security & Best Practices

### General Security
- Use dedicated keypairs for Kora (don't reuse personal wallets)
- Only fund with SOL you're willing to spend on fees
- Maintain minimal operational balance with automated monitoring and top-offs
- Implement monitoring and alerting for unusual activity
- All private keys and API keys should be stored in environment variables or secrets management systems (Railway secrets, AWS Secrets Manager, etc.)


## Specifying a Signer (Client-Side)

Clients can specify a preferred signer for consistency across related operations:

```typescript
// Fetch the signers by calling getPayerSigner
const { signer, payment_destination } = await client.getPayerSigner();
console.log(signer, payment_destination);

// Estimate with specific signer
const estimate = await client.estimateTransactionFee({
  transaction: tx,
  signer_key: signer  // Public key of preferred signer (one of the signers in the signer pool)
});

// Sign with same signer
const signed = await client.signTransaction({
  transaction: tx,
  signer_key: signer  // Same signer for consistency
});
```

Without signer keys, the configured strategy determines signer selection. It is important to note that keys must be consistent across calls related to the same transaction (e.g., if you generate a transaction with a specified signer key, you must use the same signer key for all related calls).