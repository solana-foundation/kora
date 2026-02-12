# Kora Fee Calculation Reference

## Table of Contents

- [Fee Components](#fee-components)
- [Pricing Models](#pricing-models)
- [Security Considerations](#security-considerations)

---

## Fee Components

Total fee is calculated from 5 tracked components (per `TotalFeeCalculation` in `fee/fee.rs`):

| # | Component | Description |
|---|-----------|-------------|
| 1 | **Base Fee** | Solana network fee (signatures x 5,000 lamports) + account creation rent |
| 2 | **Kora Signature Fee** | 5,000 lamports when fee payer not already a signer in the transaction |
| 3 | **Fee Payer Outflow** | SOL transfers from fee payer account (only in `margin` pricing) |
| 4 | **Payment Instruction Fee** | ~50 lamports estimate for the payment transfer instruction |
| 5 | **Transfer Fee** | Token-2022 transfer fees (if applicable) |

The margin is applied to the sum of all components.

**Formula**: `total = (base_fee + kora_sig_fee + outflow + payment_ix_fee + transfer_fee) * (1 + margin)`

---

## Pricing Models

### Margin (Default, Recommended)

```toml
[validation.price]
type = "margin"
margin = 0.1    # 10% markup
```

- Includes all fee components including fee payer outflow
- Safest model - operator always recovers costs
- `margin = 0` means exact cost pass-through

### Fixed

```toml
[validation.price]
type = "fixed"
amount = 1000        # Amount in token's smallest unit
token = "<mint>"     # Token mint address
```

- Charges flat amount regardless of actual transaction cost
- Does NOT include fee payer outflow in payment validation
- **Security risk**: Fee payer SOL can be drained if transfers not restricted

### Free

```toml
[validation.price]
type = "free"
```

- No fee charged to user
- Operator sponsors all transaction costs
- Does NOT include fee payer outflow in payment validation
- **Security risk**: Fee payer SOL can be drained if transfers not restricted

---

## Security Considerations

### Fixed/Free Pricing Vulnerability

When using `fixed` or `free` pricing, the payment validation does NOT account for fee payer outflow. A malicious transaction could include instructions that transfer SOL from the fee payer.

**Required mitigation**: Ensure all transfer operations remain `false` (the default) in fee payer policy:

```toml
[validation.fee_payer_policy.system]
allow_transfer = false
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
```

### Margin Pricing Safety

With `margin` pricing, fee payer outflow is included in the fee calculation. This means even if the fee payer is used as a transfer source, the cost is passed to the user. However, restricting unnecessary operations is still best practice.

### Monitoring

Enable balance monitoring to detect unusual fee payer drain:

```toml
[metrics]
enabled = true

[metrics.fee_payer_balance]
enabled = true
expiry_seconds = 30
```

Alert on `signer_balance_lamports` drops with: `delta(signer_balance_lamports[5m]) < -threshold`
