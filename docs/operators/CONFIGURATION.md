# Kora Configuration Reference
*Last Updated: 2025-10-28*

Your Kora node will be signing transactions for your users, so it is important to configure it to only sign transactions that meet your business requirements. Kora gives you a lot of flexibility in how you configure your node, but it is important to understand the implications of your configuration. `kora.toml` is the control center for your Kora configuration. This document provides a comprehensive reference for configuring your Kora paymaster node through the `kora.toml` configuration file.

## Overview

The `kora.toml` file controls all aspects of your Kora node's behavior including:
- Rate limiting and authentication
- RPC method availability
- Transaction validation rules
- Fee pricing models
- Security policies
- RPC method availability
- Fee pricing models
- Payment address configuration
- Performance monitoring

Your configuration file should be placed in your deployment directory or specified via the `--config` flag when starting the server.

## Configuration Sections

The `kora.toml` file is organized into sections, each with its own set of options. This guide walks through each section and explains the options available:

- [Kora Core Policies](#kora-core-policies) - Core server settings
- [Kora Authentication](#kora-authentication) - Authentication settings
- [Kora Caching](#kora-caching-optional) - Redis caching for RPC calls
- [Kora Usage Limits](#kora-usage-limits-optional) - Per-wallet transaction limiting
- [Kora Enabled Methods](#kora-enabled-methods-optional) - Kora RPC methods to enable
- [Validation Policies](#validation-policies) - Transaction validation and security
- [Token-2022 Extension Blocking](#token-2022-extension-blocking) - Block risky Token-2022 extensions
- [Fee Payer Policy](#fee-payer-policy) - Restrictions on fee payer wallet
- [Price Configuration](#price-configuration) - Transaction fee pricing models
- [Performance Monitoring](#performance-monitoring-optional) - Metrics collection and monitoring
- [Complete Example](#complete-example) - Full production-ready configuration

Sample `kora.toml` file sections:

```toml
[kora]
# Core server settings

[kora.auth]
# Authentication settings

[kora.cache]
# Redis caching configuration

[kora.usage_limit]
# Per-wallet transaction limiting

[kora.enabled_methods]
# Kora RPC methods to enable

[validation]
# Transaction validation rules

[validation.token2022]
# Token-2022 extension blocking

[validation.fee_payer_policy]
# Restrictions on fee payer wallet

[validation.price]
# Transaction fee pricing models

[metrics]
# Performance monitoring
```


## Kora Core Policies

The `[kora]` section configures core server behavior:

```toml
[kora]
rate_limit = 100
payment_address = "YourPaymentAddressPubkey11111111111111111111"  # Optional
```

| Option | Description | Required | Type |
|--------|-------------|---------|---------|
| `rate_limit` | Global rate limit (requests per second) across all clients | ✅ | number |
| `payment_address` | Optional payment address to receive payment tokens (defaults to signer address(es) if not specified) | ❌ | b58-encoded string |


## Kora Authentication

The `[kora.auth]` section configures authentication for the Kora server:

```toml
[kora.auth]
api_key = "kora_live_sk_1234567890abcdef"
hmac_secret = "kora_hmac_your-strong-hmac-secret-key-here"
max_timestamp_age = 300
```


| Option | Description | Required | Type |
|--------|-------------|---------|---------|
| `api_key` | API key for simple authentication | ❌ | string |
| `hmac_secret` | HMAC secret for signature-based authentication (min 32 chars) | ❌ | string |
| `max_timestamp_age` | Maximum age of an HMAC timestamp in seconds | ❌ (default: 300) | number |

> *Note: `api_key` and `hmac_secret` set a global authentication policy for all clients. For detailed authentication setup, see [Authentication Guide](./AUTHENTICATION.md).*

## Kora Caching (optional)

The `[kora.cache]` section configures Redis-based caching for Solana RPC calls. This can significantly improve performance by reducing redundant account data fetches:

```toml
[kora.cache]
enabled = true                      # Enable/disable caching
url = "redis://localhost:6379"      # Redis connection URL
default_ttl = 300                   # Default TTL in seconds (5 minutes)
account_ttl = 60                    # Account data TTL in seconds (1 minute)
```

| Option | Description | Required | Type |
|--------|-------------|---------|---------|
| `enabled` | Enable Redis caching for RPC calls | ❌ (default: false) | boolean |
| `url` | Redis connection URL (required when enabled) | ✅ | string |
| `default_ttl` | Default TTL for cached entries in seconds | ❌ (default: 300) | number |
| `account_ttl` | TTL for account data cache in seconds | ❌ (default: 60) | number |

> *Note: When caching is enabled, a Redis instance must be available at the specified URL. The cache gracefully falls back to direct RPC calls if Redis is unavailable.*

## Kora Usage Limits (optional)

The `[kora.usage_limit]` section configures per-wallet transaction limiting to prevent abuse and ensure fair usage across your users. This could also be used to create rewards programs to subsidize users' transaction fees up to a certain limit.
This feature requires Redis when enabled across multiple Kora instances:

```toml
[kora.usage_limit]
enabled = true                      # Enable/disable usage limiting
cache_url = "redis://localhost:6379" # Redis URL for shared state (required when enabled)
max_transactions = 100              # Max transactions per wallet (0 = unlimited)
fallback_if_unavailable = true      # Continue if Redis is unavailable
```

| Option | Description | Required | Type |
|--------|-------------|---------|---------|
| `enabled` | Enable per-wallet transaction limiting | ❌ (default: false) | boolean |
| `cache_url` | Redis connection URL for shared usage tracking | ❌ | string |
| `max_transactions` | Maximum transactions per wallet (0 = unlimited) | ❌ (default: 100) | number |
| `fallback_if_unavailable` | Allow transactions if Redis is unavailable | ❌ (default: true) | boolean |

> *Note: Usage limits are tracked per wallet address with automatic TTL-based expiration. When `fallback_if_unavailable` is true, the system allows transactions to proceed if Redis is temporarily unavailable, preventing service disruption. Setting `max_transactions` to 0 will allow unlimited transactions.*

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
get_payer_signer = true
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
    "ComputeBudget11111111111111111111111111111111", # Compute Budget Program
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

> *Note: Empty arrays are allowed, but you will need to specify at least one whitelisted `allowed_programs`, `allowed_tokens`, `allowed_spl_paid_tokens` for the Kora node to be able to process transactions. You need to specify the System Program or Token Program for the Kora node to be able to process transfers. To enable common instruction types (e.g., Compute Budget, Address Lookup Table), you need to specify the Compute Budget Program or Address Lookup Table Program, etc.*

## Token-2022 Extension Blocking

The `[validation.token2022]` section allows you to block specific Token-2022 extensions for enhanced security. All extensions are enabled by default. You can block specific extensions by adding them to the `blocked_mint_extensions` or `blocked_account_extensions` arrays:

```toml
[validation.token2022]
blocked_mint_extensions = [
    "transfer_hook",           # Block tokens with transfer hooks
    "pausable",                # Block pausable tokens
    "permanent_delegate",      # Block tokens with permanent delegates
]
blocked_account_extensions = [
    "cpi_guard",              # Block accounts with CPI guard
    "memo_transfer",          # Block accounts requiring memos
]
```

### Available Mint Extensions

| Extension Name | Description |
|----------------|-------------|
| `confidential_transfer_mint` | Confidential transfer configuration for the mint |
| `confidential_mint_burn` | Confidential mint and burn configuration |
| `transfer_fee_config` | Transfer fee configuration |
| `mint_close_authority` | Authority allowed to close the mint |
| `interest_bearing_config` | Interest-bearing token configuration |
| `non_transferable` | Makes tokens non-transferable |
| `permanent_delegate` | Permanent delegate for the mint |
| `transfer_hook` | Custom transfer hook program |
| `pausable` | Pausable token configuration |

### Available Account Extensions

| Extension Name | Description |
|----------------|-------------|
| `confidential_transfer_account` | Confidential transfer state for the account |
| `non_transferable_account` | Non-transferable token account |
| `transfer_hook_account` | Transfer hook state for the account |
| `pausable_account` | Pausable token account state |
| `memo_transfer` | Requires memo for transfers |
| `cpi_guard` | Prevents certain CPI calls |
| `immutable_owner` | Account owner cannot be changed |
| `default_account_state` | Default state for new accounts |

> *Note: Blocking extensions helps prevent interactions with tokens that have complex or potentially risky behaviors. For example, blocking `transfer_hook` prevents signing transactions for tokens with custom transfer logic.*

### Security Considerations

**PermanentDelegate Extension** - Tokens with this extension allow the delegate to transfer/burn tokens at any time without owner approval. This creates significant risks for the Kora node operator as payment funds can be seized after payment. 
- Consider adding "permanent_delegate" to `blocked_mint_extensions` in [validation.token2022] unless explicitly needed for your use case.
- Avoid using payment tokens with the `permanent_delegate` extension.

## Fee Payer Policy

The `[validation.fee_payer_policy]` section restricts what your Kora node's fee payer wallet can do. This is important to prevent unexpected behavior from users' transactions utilizing your Kora node as a signer. For example, if `allow_spl_transfers` is set to `false`, the Kora node would not sign transactions that include an SPL token transfer where the Kora node's fee payer is the instruction's authority.


```toml
[validation.fee_payer_policy]
allow_sol_transfers = false
allow_spl_transfers = false
allow_token2022_transfers = false
allow_assign = false
allow_burn = false
allow_close_account = false
allow_approve = false
``` 

| Option | Description | Required | Type |
|--------|-------------|---------|---------|
| `allow_sol_transfers` | Allow SOL transfers where the Kora node's fee payer is the signer/authority | ✅ | boolean |
| `allow_spl_transfers` | Allow SPL token transfers where the Kora node's fee payer is the signer/authority | ✅ | boolean |
| `allow_token2022_transfers` | Allow Token-2022 transfers where the Kora node's fee payer is the signer/authority | ✅ | boolean |
| `allow_assign` | Allow account ownership changes where the Kora node's fee payer is the signer/authority | ✅ | boolean |
| `allow_burn` | Allow token burn operations where the Kora node's fee payer is the signer/authority | ✅ | boolean |
| `allow_close_account` | Allow closing token accounts where the Kora node's fee payer is the signer/authority | ✅ | boolean |
| `allow_approve` | Allow token delegation/approval where the Kora node's fee payer is the signer/authority | ✅ | boolean |

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

**SECURITY WARNING:** Fixed pricing does **NOT** include fee payer outflow in the charged amount. This can allow users to drain your fee payer account if not properly configured.

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

#### Security Measures When Using Fixed/Free Pricing

1. **Disable Transfer Operations** - Prevent fee payer from being used as source in transfers:
   ```toml
   [validation.fee_payer_policy.system]
   allow_transfer = false              # Critical: Block SOL transfers
   allow_create_account = false        # Block account creation

   [validation.fee_payer_policy.spl_token]
   allow_transfer = false              # Block SPL transfers

   [validation.fee_payer_policy.token_2022]
   allow_transfer = false              # Block Token2022 transfers
   ```

2. **Enable Authentication** - Use authentication to prevent abuse:
   ```toml
   [kora.auth]
   api_key = "your-secure-api-key"
   # or
   hmac_secret = "your-minimum-32-character-hmac-secret"
   ```

3. **Set Conservative Limits** - Minimize exposure:
   ```toml
   [validation]
   max_allowed_lamports = 1000000  # 0.001 SOL maximum
   ```

## Performance Monitoring (optional)

The `[metrics]` section configures metrics collection and monitoring. This section is optional and by default, metrics are disabled.

```toml
[metrics]
enabled = true
endpoint = "/metrics"
port = 8080
scrape_interval = 60

[metrics.fee_payer_balance]
enabled = true
expiry_seconds = 30
```

| Option | Description | Required | Type |
|--------|-------------|---------|---------|
| `enabled` | Enable metrics collection | ✅ | boolean |
| `endpoint` | Custom metrics endpoint path | ✅ | string |
| `port` | Metrics endpoint port | ✅ | number |
| `scrape_interval` | Frequency of Prometheus scrape (seconds) | ✅ | number |

### Fee Payer Balance Tracking

The `[metrics.fee_payer_balance]` section configures automatic monitoring of your fee payer's SOL balance:

| Option | Description | Required | Type |
|--------|-------------|---------|---------|
| `enabled` | Enable fee payer balance tracking | ❌ (default: false) | boolean |
| `expiry_seconds` | Background tracking interval in seconds | ❌ (default: 30) | number |

When enabled, Kora automatically tracks your fee payer's SOL balance and exposes it via the `fee_payer_balance_lamports` Prometheus gauge. This helps with capacity planning and low-balance alerting.

> *Note: Metrics are served at `http://localhost:{port}/{metrics-endpoint}` (Metrics can be served on the same port as the RPC server).*

**[→ Kora Monitoring Reference Guide](./MONITORING.md)**

## Complete Example

Here's a production-ready configuration with security best practices:

```toml
# Kora Paymaster Configuration
# Last Updated: 2025-08-22

[kora]
# Rate limiting: 100 requests per second globally
rate_limit = 100

# Optional payment address (defaults to signer address(es) if not specified)
# payment_address = "YourPaymentAddressPubkey11111111111111111111"

[kora.auth]
# Authentication (choose based on security needs)
# api_key = "kora_live_sk_generate_secure_key_here"
hmac_secret = "kora_hmac_minimum_32_character_secret_here"
max_timestamp_age = 300

# Caching configuration (optional but recommended for production)
[kora.cache]
enabled = true
url = "redis://localhost:6379"
default_ttl = 300  # 5 minutes
account_ttl = 60   # 1 minute

# Usage limiting (optional, prevents abuse)
[kora.usage_limit]
enabled = true
cache_url = "redis://localhost:6379"  # Can share same Redis instance as cache
max_transactions = 100                # Per-wallet limit
fallback_if_unavailable = true        # Don't block if Redis is down

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
get_payer_signer = true

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
    "AddressLookupTab1e1111111111111111111111111",   # Address Lookup Table
    "ComputeBudget11111111111111111111111111111111", # Compute Budget
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
allow_burn = false
allow_close_account = false
allow_approve = false

# Token-2022 extension blocking
[validation.token2022]
# Block potentially risky mint extensions
blocked_mint_extensions = [
    "transfer_hook",       # Custom transfer logic
    "pausable",            # Can freeze transfers
    "permanent_delegate",  # Permanent control
]
# Block complex account extensions
blocked_account_extensions = [
    "cpi_guard",      # Restricts composability
    "memo_transfer",  # Requires additional data
]

# Sustainable pricing with 15% margin
[validation.price]
type = "margin"
margin = 0.15  # 15% margin on network fees

# Metrics collection
[metrics]
enabled = true
endpoint = "/metrics"
port = 8080
scrape_interval = 60

# Fee payer balance monitoring
[metrics.fee_payer_balance]
enabled = true
expiry_seconds = 30
```

## Configuration Validation

Kora validates your configuration on startup. If you would like to validate your configuration without starting the server, you can use the config validation command:

```bash
kora --config kora.toml config validate # or validate-with-rpc
```

You can also run the `validate-with-rpc` command to validate your configuration with the RPC server (this validation check is a little bit slower but does more thorough account checks)

## Starting the Server

Once you have configured your `kora.toml` file, you can start the Kora server:

```bash
kora --config path/to/kora.toml rpc start --no-load-signer # --other-rpc-flags-here
```

The `--no-load-signer` flag will initialize the server without loading any signers. This is useful for testing your configuration. In order to load signers, you will need to configure the `signers.toml` file. A minimum configuration with a single signer would look like this:

```toml
[signer_pool]
# Selection strategy: round_robin, random, weighted
strategy = "round_robin"

# Primary memory signer
[[signers]]
name = "my-signer"
type = "memory"
private_key_env = "MY_SIGNER_PRIVATE_KEY"
```

This will load a single signer from the `MY_SIGNER_PRIVATE_KEY` environment variable. Then you can start your server with:

```bash
kora --config path/to/kora.toml rpc start --signers-config path/to/signers.toml
```

For more information and advanced signer configuration, see the [Signers Guide](./SIGNERS.md).

## Best Practices

1. **Start Restrictive**: Begin with tight limits and gradually expand
2. **Monitor Usage**: Track which programs and tokens are actually used
3. **Regular Updates**: Review and update blocklists and limits
4. **Test Changes**: Validate configuration changes in staging first
5. **Versioning**: Keep a changelog of your configuration changes

## Need Help?

- Check [Authentication Guide](./AUTHENTICATION.md) for auth setup
- Check [Signers Guide](./SIGNERS.md) for signer configuration
- Check [Operator Guide](./README.md) for more information on how to run a Kora node
- Visit [Solana Stack Exchange](https://solana.stackexchange.com/) with the `kora` tag
- Report issues on [GitHub](https://github.com/solana-foundation/kora/issues)