# Kora Configuration Reference

Your Kora node will be signing transactions for your users, so it is important to configure it to only sign transactions that meet your business requirements. Kora gives you a lot of flexibility in how you configure your node, but it is important to understand the implications of your configuration. `kora.toml` is the control center for your Kora configuration. This document provides a comprehensive reference for configuring your Kora paymaster node through the `kora.toml` configuration file.

## Overview

The `kora.toml` file controls all aspects of your Kora node's behavior including:
- Rate limiting and authentication
- Transaction validation rules
- Fee pricing models
- Security policies
- RPC method availability

Your configuration file should be placed in your deployment directory or specified via the `--config` flag when starting the server.

## Configuration Sections

The `kora.toml` file is organized into sections, each with its own set of options. This guide walks through each section and explains the options available:

- [Kora Core Policies](#kora-core-policies) - Core server settings
- [Kora Enabled Methods](#kora-enabled-methods) - Kora RPC methods to enable
- [Validation Policies](#validation-policies) - Transaction validation and security
- [Fee Payer Policy](#fee-payer-policy) - Restrictions on fee payer wallet
- [Price Configuration](#price-configuration) - Transaction fee pricing models
- [Complete Example](#complete-example) - Full production-ready configuration

Sample `kora.toml` file sections:

```toml
[kora]
# Core server settings

[kora.enabled_methods]
# Kora RPC methods to enable

[kora.rate_limit]
# Rate limiting settings

[validation]
# Transaction validation rules

[validation.fee_payer_policy]
# Restrictions on fee payer wallet

[validation.price]
# Transaction fee pricing models
```


## Kora Core Policies

The `[kora]` section configures core server behavior:

```toml
[kora]
rate_limit = 100
api_key = "kora_live_sk_1234567890abcdef"
hmac_secret = "kora_hmac_your-strong-hmac-secret-key-here"
max_timestamp_age = 300
```


| Option | Description | Required | Type |
|--------|-------------|---------|---------|
| `rate_limit` | Global rate limit (requests per second) across all clients | ✅ | number |
| `api_key` | API key for simple authentication | ❌ | string |
| `hmac_secret` | HMAC secret for signature-based authentication (min 32 chars) | ❌ | string |
| `max_timestamp_age` | Maximum age of an HMAC timestamp in seconds | ❌ (default: 300) | number |

> *Note: `api_key` and `hmac_secret` set a global authentication policy for all clients. For detailed authentication setup, see [Authentication Guide](./AUTHENTICATION.md).*


### Kora Enabled Methods (optional)
The `[kora.enabled_methods]` section controls which RPC methods are enabled. This section is optional and by default, all methods are enabled. Each method can be enabled or disabled by setting the value to `true` or `false`:

```toml
[kora.enabled_methods]
liveness = true
estimate_transaction_fee = true
get_supported_tokens = true
sign_transaction = false
sign_and_send_transaction = false
transfer_transaction = false
get_blockhash = true
get_config = true
sign_transaction_if_paid = true
```

| Option | Method Description | Required | Type |
|--------|-------------|---------|---------|
| `liveness` | Health check endpoint | ✅ | boolean |
| `estimate_transaction_fee` | Estimate the fee for a transaction | ✅ | boolean |
| `get_supported_tokens` | List accepted tokens | ✅ | boolean |
| `sign_transaction` | Sign a transaction without sending it to the network | ✅ | boolean |
| `sign_and_send_transaction` | Sign a transaction and send it to the network | ✅ | boolean |
| `transfer_transaction` | Handle token transfers | ✅ | boolean |
| `get_blockhash` | Get a recent blockhash | ✅ | boolean |
| `get_config` | Return the Kora server config | ✅ | boolean |
| `sign_transaction_if_paid` | Conditional signing if token payment instruction is provided | ✅ | boolean |


> *Note: if this section is included in your `kora.toml` file, all methods must explicitly be set to `true` or `false`.*

## Validation Policies

The `[validation]` section defines Solana-related security rules and transaction limits:

```toml
[validation]
max_allowed_lamports = 1000000  # 0.001 SOL
max_signatures = 10
price_source = "Jupiter"
allowed_programs = [
    "11111111111111111111111111111111",              # System Program (required for SOL transfers)
    "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA",   # SPL Token Program
    "TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb",   # Token-2022 Program
    "ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL",  # Associated Token Program
    "AddressLookupTab1e1111111111111111111111111",   # Address Lookup Table Program
]
allowed_tokens = [
    "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v",  # USDC (mainnet)
    # additional tokens here
]
allowed_spl_paid_tokens = [
    "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v",  # USDC (mainnet)
    # additional tokens here
]
disallowed_accounts = [
    # "BadActorPubkey11111111111111111111111111111",
]
```

| Option | Description | Required | Type |
|--------|-------------|---------|---------|
| `max_allowed_lamports` | Setting a maximum number of lamports per transaction limits the Kora node's exposure to a single transaction. | ✅ | number |
| `max_signatures` | Solana base fees are a function of the number of signatures in a transaction, so setting a maximum number of signatures per transaction is a good way to prevent users from spending too much SOL on a single transaction. | ✅ | number |
| `price_source` | Oracle for token price data | ✅ | "Jupiter" or "Mock" |
| `allowed_programs` | Solana programs that transactions can interact with | ✅ | Array of b58-encoded string |
| `allowed_tokens` | Token mints that can be used in transactions | ✅ | Array of b58-encoded string |
| `allowed_spl_paid_tokens` | SPL tokens accepted as payment for transaction fees | ✅ | Array of b58-encoded string |
| `disallowed_accounts` | Accounts that are explicitly blocked from transactions | ✅ | Array of b58-encoded string |

> *Note: Empty arrays are allowed, but you will need to specify at least one whitelisted `allowed_programs`, `allowed_tokens`, `allowed_spl_paid_tokens` for the Kora node to be able to process transactions. You need to specify the System Program or Token Program for the Kora node to be able to process transactions.*


## Fee Payer Policy

The `[validation.fee_payer_policy]` section restricts what your Kora node's fee payer wallet can do. This is important to prevent unexpected behavior from users' transactions utilizing your Kora node as a signer. For example, if `allow_spl_transfers` is set to `false`, the Kora node would not sign transactions that include an SPL token transfer where the Kora node's fee payer is the instruction's authority.


```toml
[validation.fee_payer_policy]
allow_sol_transfers = false
allow_spl_transfers = false
allow_token2022_transfers = false
allow_assign = false
``` 

| Option | Description | Required | Type |
|--------|-------------|---------|---------|
| `allow_sol_transfers` | Allow SOL transfers where the Kora node's fee payer is the signer/authority | ✅ | boolean |
| `allow_spl_transfers` | Allow SPL token transfers where the Kora node's fee payer is the signer/authority | ✅ | boolean |
| `allow_token2022_transfers` | Allow Token-2022 transfers where the Kora node's fee payer is the signer/authority | ✅ | boolean |
| `allow_assign` | Allow account ownership changes where the Kora node's fee payer is the signer/authority | ✅ | boolean |

> *Note: For security reasons, it is recommended to set all of these to `false` and only enable as needed.*

## Price Configuration (optional)

The `[validation.price]` section defines how transaction fees are calculated. Three pricing models are available:
- Margin Pricing (default) - Add a percentage margin on top of actual network fees (default margin is 0.0)
- Fixed Pricing - Charge a fixed amount in a specific token regardless of network fees
- Free Pricing - Sponsor all transaction fees (no charge to users)

| Option | Description | Required | Type |
|--------|-------------|---------|---------|
| `type` | Pricing model to use | ✅ | "margin", "fixed" or "free" |
| `margin` | Margin percentage to add to network fees | (when `type` is "margin") | number |
| `amount` | Fixed amount to charge in token's base units | (when `type` is "fixed") | number |
| `token` | Token mint to charge in | (when `type` is "fixed") | b58-encoded string |

### Margin Pricing

Add a percentage margin on top of actual network fees:

```toml
[validation.price]
type = "margin"
margin = 0.1  # 10% margin (0.1 = 10%, 1.0 = 100%)
```

### Fixed Pricing

Charge a fixed amount in a specific token regardless of network fees:

```toml
[validation.price]
type = "fixed"
amount = 1000000  # Amount in token's base units
token = "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v"  # USDC mint
```

### Free Transactions

Sponsor all transaction fees (no charge to users):

```toml
[validation.price]
type = "free"
```

## Complete Example

Here's a production-ready configuration with security best practices:

```toml
# Kora Paymaster Configuration
# Last Updated: 2024-01-15

[kora]
# Rate limiting: 100 requests per second globally
rate_limit = 100

# Authentication (choose based on security needs)
# api_key = "kora_live_sk_generate_secure_key_here"
hmac_secret = "kora_hmac_minimum_32_character_secret_here"

# Disable unnecessary RPC methods for security
[kora.enabled_methods]
liveness = true
estimate_transaction_fee = true
get_supported_tokens = true
sign_transaction = false
sign_and_send_transaction = false
transfer_transaction = false
get_blockhash = true
get_config = true
sign_transaction_if_paid = true

[validation]
# Use production oracle
price_source = "Jupiter"

# Conservative transaction limits
max_allowed_lamports = 1000000  # 0.001 SOL max
max_signatures = 10

# Minimal program allowlist (expand as needed)
allowed_programs = [
    "11111111111111111111111111111111",              # System Program
    "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA",   # SPL Token
    "ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL",  # Associated Token
    "MyProgram111111111111111111111111111111111", 
    # Add your specific program IDs here
]

# Production token allowlist
allowed_tokens = [
    "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v",  # USDC
    "So11111111111111111111111111111111111111112",   # Wrapped SOL
    "MyToken1111111111111111111111111111111111111111",
    # Add tokens your application uses
]

# Payment tokens (only liquid, trusted tokens)
allowed_spl_paid_tokens = [
    "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v",  # USDC only
]

# Known bad actors or compromised addresses
disallowed_accounts = [
    "BadActor1111111111111111111111111111111111111111",
]

# Restrictive fee payer policy
[validation.fee_payer_policy]
allow_sol_transfers = false
allow_spl_transfers = false
allow_token2022_transfers = false
allow_assign = false

# Sustainable pricing with 15% margin
[validation.price]
type = "margin"
margin = 0.15  # 15% margin on network fees
```

## Configuration Validation

Kora validates your configuration on startup. If you would like to validate your configuration without starting the server, you can use the config validation command:

```bash
kora --config kora.toml config validate
```

## Starting the Server

Once you have configured your `kora.toml` file, you can start the Kora server:

```bash
kora --config path/to/kora.toml rpc # --other-rpc-flags-here
```

## Best Practices

1. **Start Restrictive**: Begin with tight limits and gradually expand
2. **Monitor Usage**: Track which programs and tokens are actually used
3. **Regular Updates**: Review and update blocklists and limits
4. **Test Changes**: Validate configuration changes in staging first
5. **Versioning**: Keep a changelog of your configuration changes

## Need Help?

- Check [Authentication Guide](./AUTHENTICATION.md) for auth setup
- Check [Operator Guide](./README.md) for more information on how to run a Kora node
- Visit [Solana Stack Exchange](https://solana.stackexchange.com/) with the `kora` tag
- Report issues on [GitHub](https://github.com/solana-foundation/kora/issues)