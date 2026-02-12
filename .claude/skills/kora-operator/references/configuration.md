# Kora Configuration Reference (kora.toml)

Complete reference for all kora.toml sections and fields.

## Table of Contents

- [Core Settings](#core-settings)
- [Authentication](#authentication)
- [Caching](#caching)
- [Usage Limits](#usage-limits)
- [Enabled Methods](#enabled-methods)
- [Bundle Configuration](#bundle-configuration)
- [Lighthouse Protection](#lighthouse-protection)
- [Validation](#validation)
- [Token-2022 Extensions](#token-2022-extensions)
- [Fee Payer Policy](#fee-payer-policy)
- [Pricing](#pricing)
- [Metrics](#metrics)
- [Production Example](#production-example)

---

## Core Settings

```toml
[kora]
rate_limit = 100           # Requests per second
payment_address = "<pubkey>"  # Optional: payment destination (defaults to signer address)
```

`payment_address`: If set, fee payments go to this address instead of the signer. Useful for separating signing keys from payment collection.

---

## Authentication

```toml
[kora.auth]
api_key = "your-api-key"           # or set KORA_API_KEY env var
hmac_secret = "your-hmac-secret"   # or set KORA_HMAC_SECRET env var
max_timestamp_age = 300            # HMAC timestamp validity in seconds (default: 300)
recaptcha_secret = "your-secret"   # or set KORA_RECAPTCHA_SECRET env var
recaptcha_score_threshold = 0.5    # Minimum score to pass (default: 0.5, range: 0.0-1.0)
protected_methods = [              # Methods requiring reCAPTCHA (default: signing methods)
    "sign_transaction",
    "sign_and_send_transaction",
    "sign_bundle",
    "sign_and_send_bundle",
]
```

All three methods optional. Can use simultaneously. `/liveness` always exempt.

**API Key**: Client sends `x-api-key` header.

**HMAC**: Client sends `x-timestamp` + `x-hmac-signature` (SHA256 of `timestamp + body`).

**reCAPTCHA v3**: Bot protection. Client sends `x-recaptcha-token` header. Server verifies score against threshold. Only checked on `protected_methods` (signing methods by default).

**Best practices**: 32+ char keys, regular rotation, HTTPS in production.

---

## Caching

```toml
[kora.cache]
enabled = false                    # Enable Redis caching
url = "redis://localhost:6379"     # Redis connection URL
default_ttl = 300                  # Default TTL in seconds
account_ttl = 60                   # Token account cache TTL
```

Caches token account lookups. Optional but recommended for high-throughput production.

---

## Usage Limits

Rule-based per-user limiting with Redis backend.

```toml
[kora.usage_limit]
enabled = false                          # Enable per-user limits
cache_url = "redis://redis:6379"         # Requires Redis
fallback_if_unavailable = false          # Allow transactions if Redis is down

# Transaction-level limit
[[kora.usage_limit.rules]]
type = "transaction"
max = 100
window_seconds = 3600                    # 100 transactions per hour

# Instruction-level limit (specific program + instruction)
[[kora.usage_limit.rules]]
type = "instruction"
program = "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA"
instruction = "Transfer"
max = 10
window_seconds = 3600                    # 10 token transfers per hour
```

**Rule types**:
- `transaction`: Limits total transactions per user within the time window
- `instruction`: Limits specific program instruction usage per user within the time window

Requires `user_id` parameter in signing requests when enabled with `free` pricing.

---

## Enabled Methods

```toml
[kora.enabled_methods]
liveness = true
get_version = true
estimate_transaction_fee = true
estimate_bundle_fee = true
get_supported_tokens = true
sign_transaction = true
sign_and_send_transaction = true
sign_bundle = true
sign_and_send_bundle = true
transfer_transaction = true        # Deprecated in v2.2.0
get_blockhash = true
get_config = true
get_payer_signer = true
```

All default to `true`. If section is included, ALL methods must be explicitly set.

`transfer_transaction` is deprecated — use `@solana-program/token` instructions + `signTransaction`/`signAndSendTransaction` instead.

---

## Bundle Configuration

Enable atomic multi-transaction execution via Jito block engine.

```toml
[kora.bundle]
enabled = true

[kora.bundle.jito]
block_engine_url = "https://mainnet.block-engine.jito.wtf"
```

When Jito tips are paid by Kora, set `allow_transfer = true` in `[validation.fee_payer_policy.system]`.

Enables `signBundle`, `signAndSendBundle`, and `estimateBundleFee` RPC methods. Max 5 transactions per bundle.

---

## Lighthouse Protection

Adds balance assertion instructions to protect the fee payer from drainage attacks.

```toml
[kora.lighthouse]
enabled = true
fail_if_transaction_size_overflow = true   # Fail if assertion doesn't fit in transaction
```

Only applies to `signTransaction`/`signBundle` (not broadcast methods). Requires Lighthouse program in `allowed_programs`:

```
L2TExMFKdjpN9kozasaurPirfHy9P8sbXoAN1qA3S95
```

When enabled, the returned transaction message may be modified (balance assertion added), invalidating client signatures. Client must re-sign after receiving the modified transaction.

---

## Validation

```toml
[validation]
max_allowed_lamports = 1000000     # Max transaction value in lamports
max_signatures = 10                # Max signatures per transaction
price_source = "Mock"              # "Mock" or "Jupiter" (requires JUPITER_API_KEY)
allow_durable_transactions = false # Allow durable nonce transactions (security risk!)

# Program allowlist (by public key)
allowed_programs = [
    "11111111111111111111111111111111",              # System Program
    "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA",  # Token Program
    "ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL", # Associated Token Program
    "AddressLookupTab1e1111111111111111111111111",    # Address Lookup Table
    "ComputeBudget111111111111111111111111111111",     # Compute Budget
    "MemoSq4gqABAXKb96qnH8TysNcWxMyWCqXgDLGmfcHr",  # Memo Program
]

# Token allowlists (by mint address)
allowed_tokens = ["<usdc-mint>"]
allowed_spl_paid_tokens = ["<usdc-mint>"]

# Blocked accounts
disallowed_accounts = []
```

**`price_source`**:
- `"Mock"`: Fixed 1:1 price ratio (testing/development)
- `"Jupiter"`: Live Jupiter API prices (production). Requires `JUPITER_API_KEY` env var.

**`allow_durable_transactions`**: Allows nonce-based transactions. Security risk: transactions can be replayed after nonce advance.

---

## Token-2022 Extensions

```toml
[validation.token2022]
blocked_mint_extensions = [
    "transfer_hook",
    "pausable",
    "permanent_delegate",    # Security risk: can transfer/burn any tokens
    "confidential_transfer",
    "group_pointer",
    "member_pointer",
]
blocked_account_extensions = [
    "cpi_guard",
    "memo_transfer",
]
```

Block specific Token-2022 extensions from being used in transactions.

---

## Fee Payer Policy

Controls what actions the fee payer signer can perform. All default to `false` (restrictive via Rust's `#[derive(Default)]` on bool). Explicitly enable only what you need.

```toml
# All fields default to false if omitted. Only enable what you need.
[validation.fee_payer_policy.system]
allow_transfer = false         # System Transfer/TransferWithSeed
allow_assign = false           # System Assign/AssignWithSeed
allow_create_account = false   # System CreateAccount/CreateAccountWithSeed
allow_allocate = false         # System Allocate/AllocateWithSeed

[validation.fee_payer_policy.system.nonce]
allow_initialize = false       # InitializeNonceAccount
allow_advance = false          # AdvanceNonceAccount
allow_authorize = false        # AuthorizeNonceAccount
allow_withdraw = false         # WithdrawNonceAccount

[validation.fee_payer_policy.spl_token]
allow_transfer = false         # Transfer/TransferChecked
allow_burn = false             # Burn/BurnChecked
allow_close_account = false    # CloseAccount
allow_approve = false          # Approve/ApproveChecked
allow_revoke = false           # Revoke
allow_set_authority = false    # SetAuthority
allow_mint_to = false          # MintTo/MintToChecked
allow_initialize_mint = false  # InitializeMint/InitializeMint2
allow_initialize_account = false # InitializeAccount/InitializeAccount3
allow_initialize_multisig = false # InitializeMultisig/InitializeMultisig2
allow_freeze_account = false   # FreezeAccount
allow_thaw_account = false     # ThawAccount

[validation.fee_payer_policy.token_2022]
# Same 12 fields as spl_token above, all default to false
allow_transfer = false
allow_burn = false
allow_close_account = false
allow_approve = false
allow_revoke = false
allow_set_authority = false
allow_mint_to = false
allow_initialize_mint = false
allow_initialize_account = false
allow_initialize_multisig = false
allow_freeze_account = false
allow_thaw_account = false
```

**Security note**: Since all fields default to `false`, the fee payer policy is secure by default. Only enable operations your use case requires. For `fixed`/`free` pricing, ensure transfer operations remain `false` to prevent fee payer fund drain.

---

## Pricing

Three models:

### Margin (default, recommended)

```toml
[validation.price]
type = "margin"
margin = 0.1    # 10% markup on calculated fees
```

Includes fee payer outflow in calculation. Safest option.

### Fixed

```toml
[validation.price]
type = "fixed"
amount = 1000       # Fixed amount in token's smallest unit
token = "<mint>"    # Token mint for the fixed price
strict = false      # If true, rejects transactions where actual cost exceeds fixed amount
```

Does NOT include fee payer outflow. Must secure fee payer policy.

### Free

```toml
[validation.price]
type = "free"
```

No charge. Does NOT include fee payer outflow. Must secure fee payer policy.

---

## Metrics

```toml
[metrics]
enabled = true
endpoint = "/metrics"   # HTTP path for metrics endpoint
port = 9090
scrape_interval = 15

[metrics.fee_payer_balance]
enabled = true
expiry_seconds = 60
```

Exposes Prometheus `/metrics` endpoint with:
- `kora_http_requests_total{method, status}` - Request count
- `kora_http_request_duration_seconds` - Response time percentiles
- `signer_balance_lamports{signer_name, signer_pubkey}` - SOL balance per signer

Use with Prometheus + Grafana. Run `just run-metrics` for local setup.

**Security**: Metrics endpoint is public by default. Consider firewall or auth in production.

---

## Production Example

```toml
[kora]
rate_limit = 500
payment_address = "<payment-collection-pubkey>"

[kora.auth]
api_key = "prod-api-key-32chars-minimum-here"
hmac_secret = "prod-hmac-secret-32chars-minimum"
recaptcha_secret = "your-recaptcha-v3-secret"
recaptcha_score_threshold = 0.5

[kora.cache]
enabled = true
url = "redis://redis:6379"
default_ttl = 300
account_ttl = 60

[kora.usage_limit]
enabled = true
cache_url = "redis://redis:6379"
fallback_if_unavailable = false

[[kora.usage_limit.rules]]
type = "transaction"
max = 100
window_seconds = 3600

[[kora.usage_limit.rules]]
type = "instruction"
program = "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA"
instruction = "Transfer"
max = 50
window_seconds = 3600

[kora.bundle]
enabled = true

[kora.bundle.jito]
block_engine_url = "https://mainnet.block-engine.jito.wtf"

[kora.lighthouse]
enabled = true
fail_if_transaction_size_overflow = true

[validation]
max_allowed_lamports = 5000000
max_signatures = 10
price_source = "Jupiter"
allow_durable_transactions = false

allowed_programs = [
    "11111111111111111111111111111111",
    "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA",
    "ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL",
    "ComputeBudget111111111111111111111111111111",
    "L2TExMFKdjpN9kozasaurPirfHy9P8sbXoAN1qA3S95",
]

allowed_tokens = ["EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v"]
allowed_spl_paid_tokens = ["EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v"]
disallowed_accounts = []

[validation.price]
type = "margin"
margin = 0.05

[validation.fee_payer_policy.system]
allow_transfer = true              # Required for Jito tips
allow_assign = false
allow_create_account = false
allow_allocate = false

[validation.fee_payer_policy.system.nonce]
allow_initialize = false
allow_advance = false
allow_authorize = false
allow_withdraw = false

[validation.fee_payer_policy.spl_token]
allow_transfer = false
allow_burn = false
allow_close_account = false
allow_approve = false
allow_revoke = false
allow_set_authority = false
allow_mint_to = false
allow_initialize_mint = false
allow_initialize_account = false
allow_initialize_multisig = false
allow_freeze_account = false
allow_thaw_account = false

[validation.fee_payer_policy.token_2022]
allow_transfer = false
allow_burn = false
allow_close_account = false
allow_approve = false
allow_revoke = false
allow_set_authority = false
allow_mint_to = false
allow_initialize_mint = false
allow_initialize_account = false
allow_initialize_multisig = false
allow_freeze_account = false
allow_thaw_account = false

[validation.token2022]
blocked_mint_extensions = ["permanent_delegate", "transfer_hook"]
blocked_account_extensions = []

[metrics]
enabled = true
endpoint = "/metrics"
port = 9090

[metrics.fee_payer_balance]
enabled = true
expiry_seconds = 30
```
