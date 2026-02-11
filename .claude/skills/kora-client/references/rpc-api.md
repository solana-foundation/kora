# Kora RPC API Reference

JSON-RPC 2.0 over HTTP POST. All transactions are base64-encoded.

## Table of Contents

- [estimateTransactionFee](#estimatetransactionfee)
- [signTransaction](#signtransaction)
- [signAndSendTransaction](#signandsendtransaction)
- [transferTransaction](#transfertransaction)
- [getPaymentInstruction](#getpaymentinstruction) (client-side only)
- [getConfig](#getconfig)
- [getBlockhash](#getblockhash)
- [getSupportedTokens](#getsupportedtokens)
- [getPayerSigner](#getpayersigner)
- [liveness](#liveness)

---

## estimateTransactionFee

Estimates fee in lamports and specified token.

**Request:**
```ts
{
  transaction: string;   // base64 encoded
  fee_token: string;     // token mint address
  signer_key?: string;   // optional: specific signer pubkey
  sig_verify?: boolean;  // optional: verify sigs during simulation (default: false)
}
```

**Response:**
```ts
{
  fee_in_lamports: number;
  fee_in_token: number;     // in token's smallest unit (e.g. 10^6 for USDC)
  signer_pubkey: string;
  payment_address: string;
}
```

**cURL:**
```bash
curl -X POST http://localhost:8080 -H "Content-Type: application/json" -d '{
  "jsonrpc": "2.0", "id": 1, "method": "estimateTransactionFee",
  "params": { "transaction": "<base64>", "fee_token": "<mint>" }
}'
```

---

## signTransaction

Signs transaction with Kora fee payer without broadcasting.

**Request:**
```ts
{
  transaction: string;   // base64 encoded (user should have already signed)
  signer_key?: string;
  sig_verify?: boolean;  // default: false
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

Signs transaction and broadcasts to Solana network.

**Request:** Same as `signTransaction` (sig_verify default: false).

**Response:**
```ts
{
  signed_transaction: string;  // transaction signature (hash)
  signer_pubkey: string;
}
```

---

## transferTransaction

Creates a token transfer transaction with Kora as fee payer.

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

Note: `instructions` is populated client-side by the SDK from the message. The raw RPC response does not include parsed instructions.

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
  fee_payers: string[];                    // array of signer pool public keys
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
    estimate_transaction_fee: boolean;
    get_supported_tokens: boolean;
    sign_transaction: boolean;
    sign_and_send_transaction: boolean;
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
  TransferTransactionRequest,
  SignTransactionRequest,
  SignAndSendTransactionRequest,
  EstimateTransactionFeeRequest,
  GetPaymentInstructionRequest,
  TransferTransactionResponse,
  SignTransactionResponse,
  SignAndSendTransactionResponse,
  EstimateTransactionFeeResponse,
  GetBlockhashResponse,
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
  KitTransferTransactionResponse,
  KitPaymentInstructionResponse,
} from '@solana/kora';
```

## Error Format

All methods throw on error:
```ts
throw new Error(`RPC Error ${code}: ${message}`);
```
