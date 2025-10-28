# Kora Fee Estimation Resource Guide

*Last updated: 2025-09-02*

Kora estimates transaction fees when performing `estimate_transaction_fee`, `sign_transaction` and `sign_and_send_transaction` RPC methods. To estimate fees, Kora calculates the total cost for executing transactions on Solana, including network fees, account creation costs, and optional payment processing fees. This guide breaks down each component of the fee calculation.

## Fee Calculation Formula

The main entry point for fee estimation is `FeeConfigUtil::estimate_kora_fee()` in [`crates/lib/src/fee/fee.rs`](/crates/lib/src/fee/fee.rs). It uses the following generalized formula:

```
Total Fee = Base Fee 
          + Account Creation Fee 
          + Kora Signature Fee 
          + Fee Payer Outflow 
          + Payment Instruction Fee 
          + Transfer Fee Amount
          + Price Adjustment (if configured)
```

## Fee Components

| Component | Description | Calculation Method | When Applied |
|-----------|-------------|-------------------|--------------|
| **Base Fee** | Core Solana transaction fee covering signature verification and transaction processing | `RpcClient.get_fee_for_message()` - Uses Solana's fee calculation based on compute units and priority fees | Always |
| **Account Creation Fee** | Rent-exempt minimum balance for creating new Associated Token Accounts (ATAs) | `Rent::default().minimum_balance(account_size)` - Calculates rent based on account data size (165-355 bytes depending on token extensions) | When transaction creates new ATAs |
| **Kora Signature Fee** | Additional fee when Kora signs as a non-participant fee payer | Fixed: 5,000 lamports (`LAMPORTS_PER_SIGNATURE`) | When fee payer is not already a transaction signer |
| **Fee Payer Outflow** | Total SOL the fee payer sends out in the transaction (transfers, account creations, etc.) | Sum of: System transfers from fee payer, CreateAccount funded by fee payer, Nonce withdrawals from fee payer | When fee payer performs System Program operations |
| **Payment Instruction Fee** | Estimated cost of priority fees to add a payment instruction for gasless transactions | Fixed estimate: 50 lamports (`ESTIMATED_LAMPORTS_FOR_PAYMENT_INSTRUCTION`) | When payment is required but not included in transaction |
| **Transfer Fee** | Token2022 transfer fees configured on the mint (e.g., 1% of transfer amount) | `Token2022Mint.calculate_transfer_fee(amount, epoch)` - Based on mint's transfer fee configuration | Only for Token2022 transfers to Kora payment address |
| **Price Adjustment** | Kora's pricing model markup/adjustment | Configured price model in `validation.price` - Can add markup or fixed fee amount | When `[validation.price]` is provided in kora.toml (optional) |

## Pricing Models & Fee Payer Outflow

Kora supports three pricing models that determine how users are charged for transactions:

### Margin Pricing (Default)
- **Formula:** `Total Fee = (Base + Outflow + Other Components) × (1 + margin)`
- **Includes Fee Payer Outflow:** Yes
- **Best For:** Production deployments where fees should reflect actual costs with an added operating margin

### Fixed Pricing
- **Formula:** `Total Fee = Fixed Amount (in specified token)`
- **Includes Fee Payer Outflow:** No
- **Best For:** Simplified UX with predictable pricing in controlled environments

### Free Pricing
- **Formula:** `Total Fee = 0`
- **Includes Fee Payer Outflow:** No (operator absorbs all costs)
- **Best For:** Promotional campaigns, testing, or fully sponsored applications

---

## ⚠️ Security Warning: Fixed/Free Pricing Models

**CRITICAL:** The fixed/free pricing models do **NOT** include fee payer outflow in the charged amount. This creates a significant security risk if not properly configured: If your fee payer policy allows transfers or other outflow operations, attackers can exploit this to drain your fee payer account:

### Required Security Controls

When using fixed/free pricing, you **MUST** configure restrictive fee payer policies:

```toml
[validation.fee_payer_policy.system]
allow_transfer = false              # Block SOL transfers
allow_create_account = false        # Block account creation with lamports

[validation.fee_payer_policy.spl_token]
allow_transfer = false              # Block SPL token transfers

[validation.fee_payer_policy.token_2022]
allow_transfer = false              # Block Token2022 transfers
```

### Additional Protections

1. **Enable Authentication:** Always require API key or HMAC authentication with fixed pricing
2. **Set Low Limits:** Use conservative `max_allowed_lamports` values
3. **Monitor Usage:** Track unusual patterns of high-outflow transactions
4. **Consider Margin Pricing:** Margin pricing automatically includes outflow costs

### Validation Warnings

Kora's config validator will warn you about dangerous configurations:

```bash
kora --config kora.toml config validate
```

Expected warnings for vulnerable configs:
```
⚠️  SECURITY: Fixed pricing with allow_transfer=true for System instructions.
    Users can make the fee payer transfer arbitrary SOL amounts at fixed cost.
    This can drain your fee payer account.
```
