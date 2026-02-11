# Kora RPC API Reference

JSON-RPC 2.0 over HTTP POST. All transactions are base64-encoded.

## Table of Contents

- [getVersion](#getversion)
- [estimateTransactionFee](#estimatetransactionfee)
- [estimateBundleFee](#estimatebundlefee)
- [signTransaction](#signtransaction)
- [signAndSendTransaction](#signandsendtransaction)
- [signBundle](#signbundle)
- [signAndSendBundle](#signandsendBundle)
- [transferTransaction](#transfertransaction) (deprecated)
- [getPaymentInstruction](#getpaymentinstruction) (client-side only)
- [getConfig](#getconfig)
- [getBlockhash](#getblockhash)
- [getSupportedTokens](#getsupportedtokens)
- [getPayerSigner](#getpayersigner)
- [liveness](#liveness)

---

## getVersion

Returns the Kora server version string.

**Request:** No params.

**Response:**
```ts
{ version: string; }
```

---

## estimateTransactionFee

Estimates fee in lamports and specified token.

**Request:**
```ts
{
  transaction: string;   // base64 encoded
  fee_token?: string;    // token mint address
  signer_key?: string;   // optional: specific signer pubkey
  sig_verify?: boolean;  // optional: verify sigs during simulation (default: false)
}
```

**Response:**
```ts
{
  fee_in_lamports: number;
  fee_in_token?: number;    // in token's smallest unit (e.g. 10^6 for USDC)
  signer_pubkey: string;
  payment_address: string;
}
```

---

## estimateBundleFee

Estimates total fee for a bundle of transactions (max 5).

**Request:**
```ts
{
  transactions: string[];  // array of base64 encoded transactions
  fee_token?: string;      // token mint address
  sig_verify?: boolean;    // default: false
  sign_only_indices?: number[];  // optional: estimate only specific indices
  signer_key?: string;
}
```

**Response:**
```ts
{
  total_fee_in_lamports: string;
  total_fee_in_token: string;
  fee_token: string;
  transaction_fees: Array<{
    fee_in_lamports: string;
    fee_in_token: string;
  }>;
}
```

---

## signTransaction

Signs transaction with Kora fee payer without broadcasting. Protected by reCAPTCHA when configured.

When Lighthouse is enabled, may add a balance assertion instruction (invalidates client signatures — client must re-sign).

**Request:**
```ts
{
  transaction: string;   // base64 encoded (user should have already signed)
  signer_key?: string;
  sig_verify?: boolean;  // default: false
  user_id?: string;      // for usage tracking (required when free pricing + usage limits)
}
```

**Response:**
```ts
{
  signed_transaction: string;  // base64 encoded fully-signed transaction
  signer_pubkey: string;
}
```

---

## signAndSendTransaction

Signs transaction and broadcasts to Solana network. Protected by reCAPTCHA when configured.

Lighthouse does NOT apply (would invalidate signatures before broadcast).

**Request:** Same as `signTransaction`.

**Response:**
```ts
{
  signature: string;          // transaction signature
  signed_transaction: string; // base64 encoded signed transaction
  signer_pubkey: string;
}
```

---

## signBundle

Signs a bundle of transactions (max 5) without broadcasting. Protected by reCAPTCHA when configured.

When Lighthouse is enabled, adds balance assertion to the **last transaction** in the bundle.

**Request:**
```ts
{
  transactions: string[];        // array of base64 encoded transactions
  signer_key?: string;
  sig_verify?: boolean;          // default: false
  sign_only_indices?: number[];  // optional: only sign specific indices (defaults to all)
  user_id?: string;              // for usage tracking
}
```

**Response:**
```ts
{
  signed_transactions: string[];  // array of base64 encoded signed transactions
  signer_pubkey: string;
}
```

### Partial Bundle Signing

Use `sign_only_indices` to sign only specific transactions:
```ts
const result = await client.signBundle({
  transactions: [tx1, tx2, tx3, tx4],
  sign_only_indices: [0, 2],  // only sign tx1 and tx3
});
```

---

## signAndSendBundle

Signs a bundle and submits to Jito's block engine for atomic execution. Protected by reCAPTCHA when configured.

Lighthouse does NOT apply (would invalidate signatures before broadcast).

**Request:** Same as `signBundle`.

**Response:**
```ts
{
  bundle_uuid: string;           // Jito bundle UUID for tracking
  signed_transactions: string[]; // array of base64 encoded signed transactions
  signer_pubkey: string;
}
```

Note: When using Jito tips paid by Kora, ensure `allow_transfer = true` in `[validation.fee_payer_policy.system]`.

---

## transferTransaction

> **Deprecated** as of Kora v2.2.0. Returns payment instruction data without signing. Use `@solana-program/token` instructions + `signTransaction`/`signAndSendTransaction` instead.

**Request:**
```ts
{
  amount: number;         // in token's smallest unit
  token: string;          // mint address ("11111111111111111111111111111111" for SOL)
  source: string;         // source wallet pubkey (not token account)
  destination: string;    // destination wallet pubkey (not token account)
  signer_key?: string;
}
```

**Response:**
```ts
{
  transaction: string;       // base64 encoded
  message: string;           // base64 encoded message
  blockhash: string;
  signer_pubkey: string;
  instructions: Instruction[];  // parsed instructions (SDK only, populated client-side)
}
```

---

## getPaymentInstruction

**Client-side only** - no actual RPC call. Calls `estimateTransactionFee` internally and constructs a token transfer instruction to pay Kora.

**Request:**
```ts
{
  transaction: string;         // base64 encoded estimate transaction
  fee_token: string;           // mint address for fee payment
  source_wallet: string;       // wallet owner paying fees
  token_program_id?: string;   // defaults to TOKEN_PROGRAM_ADDRESS
  signer_key?: string;
  sig_verify?: boolean;        // default: false
}
```

**Response:**
```ts
{
  original_transaction: string;
  payment_instruction: Instruction;  // SPL token transfer instruction to append
  payment_amount: number;
  payment_token: string;
  payment_address: string;
  signer_address: string;
}
```

---

## getConfig

Returns server configuration including fee payers, validation rules, and enabled methods.

**Request:** No params.

**Response:**
```ts
{
  fee_payers: string[];
  validation_config: {
    max_allowed_lamports: number;
    max_signatures: number;
    price_source: 'Jupiter' | 'Mock';
    allowed_programs: string[];
    allowed_tokens: string[];
    allowed_spl_paid_tokens: string[];
    disallowed_accounts: string[];
    fee_payer_policy: FeePayerPolicy;
    price: PriceConfig;
    token2022: Token2022Config;
  };
  enabled_methods: {
    liveness: boolean;
    get_version: boolean;
    estimate_transaction_fee: boolean;
    estimate_bundle_fee: boolean;
    get_supported_tokens: boolean;
    sign_transaction: boolean;
    sign_and_send_transaction: boolean;
    sign_bundle: boolean;
    sign_and_send_bundle: boolean;
    transfer_transaction: boolean;
    get_blockhash: boolean;
    get_config: boolean;
  };
}
```

---

## getBlockhash

**Request:** No params.

**Response:**
```ts
{ blockhash: string; }  // base58 encoded
```

---

## getSupportedTokens

**Request:** No params.

**Response:**
```ts
{ tokens: string[]; }  // array of mint addresses
```

---

## getPayerSigner

Returns the recommended signer and payment destination.

**Request:** No params.

**Response:**
```ts
{
  signer_address: string;
  payment_address: string;
}
```

---

## liveness

Health check. Returns HTTP 200. Bypasses authentication.

---

## TypeScript Types

All request/response types are exported from `@solana/kora`:

```ts
import type {
  SignTransactionRequest,
  SignAndSendTransactionRequest,
  SignBundleRequest,
  SignAndSendBundleRequest,
  EstimateTransactionFeeRequest,
  EstimateBundleFeeRequest,
  GetPaymentInstructionRequest,
  SignTransactionResponse,
  SignAndSendTransactionResponse,
  SignBundleResponse,
  SignAndSendBundleResponse,
  EstimateTransactionFeeResponse,
  EstimateBundleFeeResponse,
  GetBlockhashResponse,
  GetVersionResponse,
  GetSupportedTokensResponse,
  GetPayerSignerResponse,
  GetPaymentInstructionResponse,
  Config,
  ValidationConfig,
  EnabledMethods,
  FeePayerPolicy,
  PriceConfig,
  PriceModel,
  KoraClientOptions,
} from '@solana/kora';
```

Kit plugin types (Address, Blockhash typed):
```ts
import type {
  KoraPluginConfig,
  KoraApi,
  KitConfigResponse,
  KitPayerSignerResponse,
  KitBlockhashResponse,
  KitSupportedTokensResponse,
  KitEstimateFeeResponse,
  KitSignTransactionResponse,
  KitSignAndSendTransactionResponse,
  KitPaymentInstructionResponse,
  KitSignBundleResponse,
  KitSignAndSendBundleResponse,
  KitEstimateBundleFeeResponse,
} from '@solana/kora';
```

## Error Format

All methods throw on error:
```ts
throw new Error(`RPC Error ${code}: ${message}`);
```
