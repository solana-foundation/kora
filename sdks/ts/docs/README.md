# Kora TypeScript SDK v0.1.0

## Classes

### KoraClient

Kora RPC client for interacting with the Kora paymaster service.

Provides methods to estimate fees, sign transactions, and perform gasless transfers
on Solana as specified by the Kora paymaster operator.

#### Example

```typescript
const client = new KoraClient({ 
  rpcUrl: 'http://localhost:8080',
  // apiKey may be required by some operators
  // apiKey: 'your-api-key',
  // hmacSecret may be required by some operators
  // hmacSecret: 'your-hmac-secret'
});

// Sample usage: Get config
const config = await client.getConfig();
```

## Methods

- [estimateTransactionFee()](#estimatetransactionfee)
- [getBlockhash()](#getblockhash)
- [getConfig()](#getconfig)
- [getSupportedTokens()](#getsupportedtokens)
- [signAndSendTransaction()](#signandsendtransaction)
- [signTransaction()](#signtransaction)
- [signTransactionIfPaid()](#signtransactionifpaid)
- [transferTransaction()](#transfertransaction)

#### Constructors

##### Constructor

```ts
new KoraClient(options: KoraClientOptions): KoraClient;
```

Creates a new Kora client instance.

###### Parameters

| Parameter | Type | Description |
| ------ | ------ | ------ |
| `options` | [`KoraClientOptions`](#koraclientoptions) | Client configuration options |

###### Returns

[`KoraClient`](#koraclient)

#### Methods

##### estimateTransactionFee()

```ts
estimateTransactionFee(request: EstimateTransactionFeeRequest): Promise<EstimateTransactionFeeResponse>;
```

Estimates the transaction fee in both lamports and the specified token.

###### Parameters

| Parameter | Type | Description |
| ------ | ------ | ------ |
| `request` | [`EstimateTransactionFeeRequest`](#estimatetransactionfeerequest) | Fee estimation request parameters |

###### Returns

`Promise`\<[`EstimateTransactionFeeResponse`](#estimatetransactionfeeresponse)\>

Fee amounts in both lamports and the specified token

###### Throws

When the RPC call fails, the transaction is invalid, or the token is not supported

###### Example

```typescript
const fees = await client.estimateTransactionFee({
  transaction: 'base64EncodedTransaction',
  fee_token: 'EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v' // USDC
});
console.log('Fee in lamports:', fees.fee_in_lamports);
console.log('Fee in USDC:', fees.fee_in_token);
```

##### getBlockhash()

```ts
getBlockhash(): Promise<GetBlockhashResponse>;
```

Gets the latest blockhash from the Solana RPC that the Kora server is connected to.

###### Returns

`Promise`\<[`GetBlockhashResponse`](#getblockhashresponse)\>

Object containing the current blockhash

###### Throws

When the RPC call fails

###### Example

```typescript
const { blockhash } = await client.getBlockhash();
console.log('Current blockhash:', blockhash);
```

##### getConfig()

```ts
getConfig(): Promise<Config>;
```

Retrieves the current Kora server configuration.

###### Returns

`Promise`\<[`Config`](#config)\>

The server configuration including fee payer address and validation rules

###### Throws

When the RPC call fails

###### Example

```typescript
const config = await client.getConfig();
console.log('Fee payer:', config.fee_payer);
console.log('Validation config:', JSON.stringify(config.validation_config, null, 2));
```

##### getSupportedTokens()

```ts
getSupportedTokens(): Promise<GetSupportedTokensResponse>;
```

Retrieves the list of tokens supported for fee payment.

###### Returns

`Promise`\<[`GetSupportedTokensResponse`](#getsupportedtokensresponse)\>

Object containing an array of supported token mint addresses

###### Throws

When the RPC call fails

###### Example

```typescript
const { tokens } = await client.getSupportedTokens();
console.log('Supported tokens:', tokens);
// Output: ['EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v', ...]
```

##### signAndSendTransaction()

```ts
signAndSendTransaction(request: SignAndSendTransactionRequest): Promise<SignAndSendTransactionResponse>;
```

Signs a transaction and immediately broadcasts it to the Solana network.

###### Parameters

| Parameter | Type | Description |
| ------ | ------ | ------ |
| `request` | [`SignAndSendTransactionRequest`](#signandsendtransactionrequest) | Sign and send request parameters |

###### Returns

`Promise`\<[`SignAndSendTransactionResponse`](#signandsendtransactionresponse)\>

Signature and the signed transaction

###### Throws

When the RPC call fails, validation fails, or broadcast fails

###### Example

```typescript
const result = await client.signAndSendTransaction({
  transaction: 'base64EncodedTransaction'
});
console.log('Transaction signature:', result.signature);
```

##### signTransaction()

```ts
signTransaction(request: SignTransactionRequest): Promise<SignTransactionResponse>;
```

Signs a transaction with the Kora fee payer without broadcasting it.

###### Parameters

| Parameter | Type | Description |
| ------ | ------ | ------ |
| `request` | [`SignTransactionRequest`](#signtransactionrequest) | Sign request parameters |

###### Returns

`Promise`\<[`SignTransactionResponse`](#signtransactionresponse)\>

Signature and the signed transaction

###### Throws

When the RPC call fails or transaction validation fails

###### Example

```typescript
const result = await client.signTransaction({
  transaction: 'base64EncodedTransaction'
});
console.log('Signature:', result.signature);
console.log('Signed tx:', result.signed_transaction);
```

##### signTransactionIfPaid()

```ts
signTransactionIfPaid(request: SignTransactionIfPaidRequest): Promise<SignTransactionIfPaidResponse>;
```

Signs a transaction only if it includes proper payment to the fee payer.

###### Parameters

| Parameter | Type | Description |
| ------ | ------ | ------ |
| `request` | [`SignTransactionIfPaidRequest`](#signtransactionifpaidrequest) | Conditional sign request parameters |

###### Returns

`Promise`\<[`SignTransactionIfPaidResponse`](#signtransactionifpaidresponse)\>

The original and signed transaction

###### Throws

When the RPC call fails or payment validation fails

###### Example

```typescript
const result = await client.signTransactionIfPaid({
  transaction: 'base64EncodedTransaction'
});
console.log('Signed transaction:', result.signed_transaction);
```

##### transferTransaction()

```ts
transferTransaction(request: TransferTransactionRequest): Promise<TransferTransactionResponse>;
```

Creates a token transfer transaction with Kora as the fee payer.

###### Parameters

| Parameter | Type | Description |
| ------ | ------ | ------ |
| `request` | [`TransferTransactionRequest`](#transfertransactionrequest) | Transfer request parameters |

###### Returns

`Promise`\<[`TransferTransactionResponse`](#transfertransactionresponse)\>

Base64-encoded signed transaction, base64-encoded message, and blockhash

###### Throws

When the RPC call fails or token is not supported

###### Example

```typescript
const transfer = await client.transferTransaction({
  amount: 1000000, // 1 USDC (6 decimals)
  token: 'EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v',
  source: 'sourceWalletPublicKey',
  destination: 'destinationWalletPublicKey'
});
console.log('Transaction:', transfer.transaction);
console.log('Message:', transfer.message);
```

## Interfaces

### AuthenticationHeaders

Authentication headers for API requests.

#### Properties

| Property | Type | Description |
| ------ | ------ | ------ |
| <a id="x-api-key"></a> `x-api-key?` | `string` | API key for simple authentication |
| <a id="x-hmac-signature"></a> `x-hmac-signature?` | `string` | HMAC SHA256 signature of timestamp + body |
| <a id="x-timestamp"></a> `x-timestamp?` | `string` | Unix timestamp for HMAC authentication |

***

### Config

Kora server configuration.

#### Properties

| Property | Type | Description |
| ------ | ------ | ------ |
| <a id="fee_payer"></a> `fee_payer` | `string` | Public key of the fee payer account |
| <a id="validation_config"></a> `validation_config` | [`ValidationConfig`](#validationconfig) | Validation rules and constraints |

***

### EstimateTransactionFeeRequest

Parameters for estimating transaction fees.

#### Properties

| Property | Type | Description |
| ------ | ------ | ------ |
| <a id="fee_token"></a> `fee_token` | `string` | Mint address of the token to calculate fees in |
| <a id="transaction"></a> `transaction` | `string` | Base64-encoded transaction to estimate fees for |

***

### EstimateTransactionFeeResponse

Response containing estimated transaction fees.

#### Properties

| Property | Type | Description |
| ------ | ------ | ------ |
| <a id="fee_in_lamports"></a> `fee_in_lamports` | `number` | Transaction fee in lamports |
| <a id="fee_in_token"></a> `fee_in_token` | `number` | Transaction fee in the requested token (in decimals value of the token, e.g. 10^6 for USDC) |

***

### FeePayerPolicy

Policy controlling what actions the fee payer can perform.

#### Properties

| Property | Type | Description |
| ------ | ------ | ------ |
| <a id="allow_assign"></a> `allow_assign` | `boolean` | Allow fee payer to use Assign instruction |
| <a id="allow_sol_transfers"></a> `allow_sol_transfers` | `boolean` | Allow fee payer to be source in SOL transfers |
| <a id="allow_spl_transfers"></a> `allow_spl_transfers` | `boolean` | Allow fee payer to be source in SPL token transfers |
| <a id="allow_token2022_transfers"></a> `allow_token2022_transfers` | `boolean` | Allow fee payer to be source in Token2022 transfers |

***

### GetBlockhashResponse

Response containing the latest blockhash.

#### Properties

| Property | Type | Description |
| ------ | ------ | ------ |
| <a id="blockhash"></a> `blockhash` | `string` | Base58-encoded blockhash |

***

### GetSupportedTokensResponse

Response containing supported token mint addresses.

#### Properties

| Property | Type | Description |
| ------ | ------ | ------ |
| <a id="tokens"></a> `tokens` | `string`[] | Array of supported token mint addresses |

***

### KoraClientOptions

Options for initializing a Kora client.

#### Properties

| Property | Type | Description |
| ------ | ------ | ------ |
| <a id="apikey"></a> `apiKey?` | `string` | Optional API key for authentication |
| <a id="hmacsecret"></a> `hmacSecret?` | `string` | Optional HMAC secret for signature-based authentication |
| <a id="rpcurl"></a> `rpcUrl` | `string` | URL of the Kora RPC server |

***

### RpcError

JSON-RPC error object.

#### Properties

| Property | Type | Description |
| ------ | ------ | ------ |
| <a id="code"></a> `code` | `number` | Error code |
| <a id="message"></a> `message` | `string` | Human-readable error message |

***

### RpcRequest\<T\>

JSON-RPC request structure.

#### Type Parameters

| Type Parameter | Description |
| ------ | ------ |
| `T` | Type of the params object |

#### Properties

| Property | Type | Description |
| ------ | ------ | ------ |
| <a id="id"></a> `id` | `number` | Request ID |
| <a id="jsonrpc"></a> `jsonrpc` | `"2.0"` | JSON-RPC version |
| <a id="method"></a> `method` | `string` | RPC method name |
| <a id="params"></a> `params` | `T` | Method parameters |

***

### SignAndSendTransactionRequest

Parameters for signing and sending a transaction.

#### Properties

| Property | Type | Description |
| ------ | ------ | ------ |
| <a id="transaction-1"></a> `transaction` | `string` | Base64-encoded transaction to sign and send |

***

### SignAndSendTransactionResponse

Response from signing and sending a transaction.

#### Properties

| Property | Type | Description |
| ------ | ------ | ------ |
| <a id="signature"></a> `signature` | `string` | Base58-encoded transaction signature |
| <a id="signed_transaction"></a> `signed_transaction` | `string` | Base64-encoded signed transaction |

***

### SignTransactionIfPaidRequest

Parameters for conditionally signing a transaction based on payment.

#### Properties

| Property | Type | Description |
| ------ | ------ | ------ |
| <a id="transaction-2"></a> `transaction` | `string` | Base64-encoded transaction |

***

### SignTransactionIfPaidResponse

Response from conditionally signing a transaction.

#### Properties

| Property | Type | Description |
| ------ | ------ | ------ |
| <a id="signed_transaction-1"></a> `signed_transaction` | `string` | Base64-encoded signed transaction |
| <a id="transaction-3"></a> `transaction` | `string` | Base64-encoded original transaction |

***

### SignTransactionRequest

Parameters for signing a transaction.

#### Properties

| Property | Type | Description |
| ------ | ------ | ------ |
| <a id="transaction-4"></a> `transaction` | `string` | Base64-encoded transaction to sign |

***

### SignTransactionResponse

Response from signing a transaction.

#### Properties

| Property | Type | Description |
| ------ | ------ | ------ |
| <a id="signature-1"></a> `signature` | `string` | Base58-encoded signature |
| <a id="signed_transaction-2"></a> `signed_transaction` | `string` | Base64-encoded signed transaction |

***

### TransferTransactionRequest

Parameters for creating a token transfer transaction.

#### Properties

| Property | Type | Description |
| ------ | ------ | ------ |
| <a id="amount"></a> `amount` | `number` | Amount to transfer in the token's smallest unit (e.g., lamports for SOL) |
| <a id="destination"></a> `destination` | `string` | Public key of the destination wallet (not token account) |
| <a id="source"></a> `source` | `string` | Public key of the source wallet (not token account) |
| <a id="token"></a> `token` | `string` | Mint address of the token to transfer |

***

### TransferTransactionResponse

Response from creating a transfer transaction.

#### Properties

| Property | Type | Description |
| ------ | ------ | ------ |
| <a id="blockhash-1"></a> `blockhash` | `string` | Recent blockhash used in the transaction |
| <a id="message-1"></a> `message` | `string` | Base64-encoded message |
| <a id="transaction-5"></a> `transaction` | `string` | Base64-encoded signed transaction |

***

### ValidationConfig

Validation configuration for the Kora server.

#### Properties

| Property | Type | Description |
| ------ | ------ | ------ |
| <a id="allowed_programs"></a> `allowed_programs` | `string`[] | List of allowed Solana program IDs |
| <a id="allowed_spl_paid_tokens"></a> `allowed_spl_paid_tokens` | `string`[] | List of SPL tokens accepted for paid transactions |
| <a id="allowed_tokens"></a> `allowed_tokens` | `string`[] | List of allowed token mint addresses for fee payment |
| <a id="disallowed_accounts"></a> `disallowed_accounts` | `string`[] | List of blocked account addresses |
| <a id="fee_payer_policy"></a> `fee_payer_policy` | [`FeePayerPolicy`](#feepayerpolicy) | Policy controlling fee payer permissions |
| <a id="max_allowed_lamports"></a> `max_allowed_lamports` | `number` | Maximum allowed transaction value in lamports |
| <a id="max_signatures"></a> `max_signatures` | `number` | Maximum number of signatures allowed per transaction |
| <a id="price"></a> `price` | [`PriceModel`](#pricemodel) | Pricing model configuration |
| <a id="price_source"></a> `price_source` | `"Jupiter"` \| `"Mock"` | Price oracle source for token conversions |

## Type Aliases

### PriceConfig

```ts
type PriceConfig = PriceModel;
```

***

### PriceModel

```ts
type PriceModel = 
  | {
  margin: number;
  type: "margin";
}
  | {
  amount: number;
  token: string;
  type: "fixed";
}
  | {
  type: "free";
};
```

Pricing model for transaction fees.

#### Remarks

- `margin`: Adds a percentage margin to base fees
- `fixed`: Charges a fixed amount in a specific token
- `free`: No additional fees charged
