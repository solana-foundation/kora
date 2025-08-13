import {
  Config,
  EstimateTransactionFeeRequest,
  EstimateTransactionFeeResponse,
  GetBlockhashResponse,
  GetSupportedTokensResponse,
  SignAndSendTransactionRequest,
  SignAndSendTransactionResponse,
  SignTransactionIfPaidRequest,
  SignTransactionIfPaidResponse,
  SignTransactionRequest,
  SignTransactionResponse,
  TransferTransactionRequest,
  TransferTransactionResponse,
  RpcError,
  RpcRequest,
  AuthenticationHeaders,
  KoraClientOptions,
  AppendPaymentInstructionRequest,
  AppendPaymentInstructionResponse,
} from "./types/index.js";
import crypto from "crypto";
import { getTransferCheckedInstruction, findAssociatedTokenPda } from "@solana-program/token-2022";
import { appendTransactionMessageInstruction, getBase64EncodedWireTransaction, createNoopSigner, getBase64Codec, getTransactionCodec, getCompiledTransactionMessageCodec, decompileTransactionMessage, compileTransactionMessage, type TransactionMessageBytes, Address, address, assertIsAddress } from "@solana/kit";

/**
 * Kora RPC client for interacting with the Kora paymaster service.
 * 
 * Provides methods to estimate fees, sign transactions, and perform gasless transfers
 * on Solana as specified by the Kora paymaster operator.
 * 
 * @example Kora Initialization
 * ```typescript
 * const client = new KoraClient({ 
 *   rpcUrl: 'http://localhost:8080',
 *   // apiKey may be required by some operators
 *   // apiKey: 'your-api-key',
 *   // hmacSecret may be required by some operators
 *   // hmacSecret: 'your-hmac-secret'
 * });
 * 
 * // Sample usage: Get config
 * const config = await client.getConfig();
 * ```
 */
export class KoraClient {
  private rpcUrl: string;
  private apiKey?: string;
  private hmacSecret?: string;

  /**
   * Creates a new Kora client instance.
   * @param options - Client configuration options
   * @param options.rpcUrl - The Kora RPC server URL
   * @param options.apiKey - Optional API key for authentication
   * @param options.hmacSecret - Optional HMAC secret for signature-based authentication
   */
  constructor({ rpcUrl, apiKey, hmacSecret }: KoraClientOptions) {
    this.rpcUrl = rpcUrl;
    this.apiKey = apiKey;
    this.hmacSecret = hmacSecret;
  }

  private getHmacSignature({
    timestamp,
    body,
  }: {
    timestamp: string;
    body: string;
  }): string {
    if (!this.hmacSecret) {
      throw new Error("HMAC secret is not set");
    }
    const message = timestamp + body;
    return crypto
      .createHmac("sha256", this.hmacSecret)
      .update(message)
      .digest("hex");
  }

  private getHeaders({ body }: { body: string }): AuthenticationHeaders {
    const headers: AuthenticationHeaders = {};
    if (this.apiKey) {
      headers["x-api-key"] = this.apiKey;
    }
    if (this.hmacSecret) {
      const timestamp = Math.floor(Date.now() / 1000).toString();
      const signature = this.getHmacSignature({ timestamp, body });
      headers["x-timestamp"] = timestamp;
      headers["x-hmac-signature"] = signature;
    }
    return headers;
  }

  private async rpcRequest<T, U>(method: string, params: U): Promise<T> {
    const body = JSON.stringify({
      jsonrpc: "2.0",
      id: 1,
      method,
      params,
    });
    const headers = this.getHeaders({ body });
    const response = await fetch(this.rpcUrl, {
      method: "POST",
      headers: { ...headers, "Content-Type": "application/json" },
      body: JSON.stringify({
        jsonrpc: "2.0",
        id: 1,
        method,
        params,
      } as RpcRequest<U>),
    });

    const json = (await response.json()) as { error?: RpcError; result: T };

    if (json.error) {
      const error = json.error!;
      throw new Error(`RPC Error ${error.code}: ${error.message}`);
    }

    return json.result;
  }

  /**
   * Retrieves the current Kora server configuration.
   * @returns The server configuration including fee payer address and validation rules
   * @throws {Error} When the RPC call fails
   * 
   * @example
   * ```typescript
   * const config = await client.getConfig();
   * console.log('Fee payer:', config.fee_payer);
   * console.log('Validation config:', JSON.stringify(config.validation_config, null, 2));
   * ```
   */
  async getConfig(): Promise<Config> {
    return this.rpcRequest<Config, undefined>("getConfig", undefined);
  }

  /**
   * Gets the latest blockhash from the Solana RPC that the Kora server is connected to.
   * @returns Object containing the current blockhash
   * @throws {Error} When the RPC call fails
   * 
   * @example
   * ```typescript
   * const { blockhash } = await client.getBlockhash();
   * console.log('Current blockhash:', blockhash);
   * ```
   */
  async getBlockhash(): Promise<GetBlockhashResponse> {
    return this.rpcRequest<GetBlockhashResponse, undefined>(
      "getBlockhash",
      undefined
    );
  }

  /**
   * Retrieves the list of tokens supported for fee payment.
   * @returns Object containing an array of supported token mint addresses
   * @throws {Error} When the RPC call fails
   * 
   * @example
   * ```typescript
   * const { tokens } = await client.getSupportedTokens();
   * console.log('Supported tokens:', tokens);
   * // Output: ['EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v', ...]
   * ```
   */
  async getSupportedTokens(): Promise<GetSupportedTokensResponse> {
    return this.rpcRequest<GetSupportedTokensResponse, undefined>(
      "getSupportedTokens",
      undefined
    );
  }

  /**
   * Estimates the transaction fee in both lamports and the specified token.
   * @param request - Fee estimation request parameters
   * @param request.transaction - Base64-encoded transaction to estimate fees for
   * @param request.fee_token - Mint address of the token to calculate fees in
   * @returns Fee amounts in both lamports and the specified token
   * @throws {Error} When the RPC call fails, the transaction is invalid, or the token is not supported
   * 
   * @example
   * ```typescript
   * const fees = await client.estimateTransactionFee({
   *   transaction: 'base64EncodedTransaction',
   *   fee_token: 'EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v' // USDC
   * });
   * console.log('Fee in lamports:', fees.fee_in_lamports);
   * console.log('Fee in USDC:', fees.fee_in_token);
   * ```
   */
  async estimateTransactionFee(
    request: EstimateTransactionFeeRequest
  ): Promise<EstimateTransactionFeeResponse> {
    return this.rpcRequest<
      EstimateTransactionFeeResponse,
      EstimateTransactionFeeRequest
    >("estimateTransactionFee", request);
  }

  /**
   * Signs a transaction with the Kora fee payer without broadcasting it.
   * @param request - Sign request parameters
   * @param request.transaction - Base64-encoded transaction to sign
   * @returns Signature and the signed transaction
   * @throws {Error} When the RPC call fails or transaction validation fails
   * 
   * @example
   * ```typescript
   * const result = await client.signTransaction({
   *   transaction: 'base64EncodedTransaction'
   * });
   * console.log('Signature:', result.signature);
   * console.log('Signed tx:', result.signed_transaction);
   * ```
   */
  async signTransaction(
    request: SignTransactionRequest
  ): Promise<SignTransactionResponse> {
    return this.rpcRequest<SignTransactionResponse, SignTransactionRequest>(
      "signTransaction",
      request
    );
  }

  /**
   * Signs a transaction and immediately broadcasts it to the Solana network.
   * @param request - Sign and send request parameters
   * @param request.transaction - Base64-encoded transaction to sign and send
   * @returns Signature and the signed transaction
   * @throws {Error} When the RPC call fails, validation fails, or broadcast fails
   * 
   * @example
   * ```typescript
   * const result = await client.signAndSendTransaction({
   *   transaction: 'base64EncodedTransaction'
   * });
   * console.log('Transaction signature:', result.signature);
   * ```
   */
  async signAndSendTransaction(
    request: SignAndSendTransactionRequest
  ): Promise<SignAndSendTransactionResponse> {
    return this.rpcRequest<
      SignAndSendTransactionResponse,
      SignAndSendTransactionRequest
    >("signAndSendTransaction", request);
  }

  /**
   * Signs a transaction only if it includes proper payment to the fee payer.
   * @param request - Conditional sign request parameters  
   * @param request.transaction - Base64-encoded transaction to conditionally sign
   * @returns The original and signed transaction
   * @throws {Error} When the RPC call fails or payment validation fails
   * 
   * @example
   * ```typescript
   * const result = await client.signTransactionIfPaid({
   *   transaction: 'base64EncodedTransaction'
   * });
   * console.log('Signed transaction:', result.signed_transaction);
   * ```
   */
  async signTransactionIfPaid(
    request: SignTransactionIfPaidRequest
  ): Promise<SignTransactionIfPaidResponse> {
    return this.rpcRequest<
      SignTransactionIfPaidResponse,
      SignTransactionIfPaidRequest
    >("signTransactionIfPaid", request);
  }

  /**
   * Creates a token transfer transaction with Kora as the fee payer.
   * @param request - Transfer request parameters
   * @param request.amount - Amount to transfer (in token's smallest unit)
   * @param request.token - Mint address of the token to transfer
   * @param request.source - Source wallet public key
   * @param request.destination - Destination wallet public key
   * @returns Base64-encoded signed transaction, base64-encoded message, and blockhash
   * @throws {Error} When the RPC call fails or token is not supported
   * 
   * @example
   * ```typescript
   * const transfer = await client.transferTransaction({
   *   amount: 1000000, // 1 USDC (6 decimals)
   *   token: 'EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v',
   *   source: 'sourceWalletPublicKey',
   *   destination: 'destinationWalletPublicKey'
   * });
   * console.log('Transaction:', transfer.transaction);
   * console.log('Message:', transfer.message);
   * ```
   */
  async transferTransaction(
    request: TransferTransactionRequest
  ): Promise<TransferTransactionResponse> {
    return this.rpcRequest<
      TransferTransactionResponse,
      TransferTransactionRequest
    >("transferTransaction", request);
  }


  private requiresPayment(config: Config): boolean {
    return config.validation_config.price.type === "fixed" || config.validation_config.price.type === "margin";
  }
  private supportsToken(config: Config, token: string): boolean {
    return config.validation_config.allowed_spl_paid_tokens.includes(token);
  }

  async appendPaymentInstruction(
    { transaction, fee_token, source_wallet, token_program_id }: AppendPaymentInstructionRequest
  ): Promise<AppendPaymentInstructionResponse> {
    assertIsAddress(source_wallet);
    assertIsAddress(fee_token);
    assertIsAddress(token_program_id);
    // first make sure token is supported
    const config = await this.getConfig();
    if (!this.supportsToken(config, fee_token)) {
      throw new Error(`Token ${fee_token} is not supported`);
    }
    // get the dude's fee payer address
    const feePayer = config.fee_payer;
    if (!this.requiresPayment(config)) {
      throw new Error("This transaction does not require payment");
    }

    const koraPayer = address(config.fee_payer);

    // then estimate the fee
    const fee = await this.estimateTransactionFee({ transaction, fee_token });

    // get associated token accounts
    const [sourceTokenAccount] = await findAssociatedTokenPda({
      owner: source_wallet,
      tokenProgram: token_program_id,
      mint: fee_token,
    });
    const [destinationTokenAccount] = await findAssociatedTokenPda({
      owner: config.fee_payer as Address,
      tokenProgram: token_program_id,
      mint: fee_token,
    });

    const paymentInstruction = await getTransferCheckedInstruction({
      source: sourceTokenAccount,
      mint: fee_token,
      destination: destinationTokenAccount,
      authority: createNoopSigner(source_wallet),
      amount: fee.fee_in_token,
      decimals: 6,
    }, { programAddress: token_program_id });

    const transactionBytes = getBase64Codec().encode(transaction);
    const originalTransaction = getTransactionCodec().decode(transactionBytes);
    const originalMessage = getCompiledTransactionMessageCodec().decode(originalTransaction.messageBytes);
    const decompiledMessage = decompileTransactionMessage(originalMessage);
    const newMessage = appendTransactionMessageInstruction(paymentInstruction, decompiledMessage);
    // Create the new transaction with payment instruction appended
    // We need to cast here because the returned type from appendTransactionMessageInstruction
    // is more specific than what compileTransactionMessage expects, but they are compatible
    // safe stringify for bigint
    console.log("NEW MESSAGE ACCOUNTS", JSON.stringify(newMessage.instructions[1].accounts, (key, value) => {
      if (typeof value === 'bigint') {
        return value.toString();
      }
      return value;
    }, 2));
    const compiledMessage = compileTransactionMessage(newMessage as any);
    const encodedMessage = getCompiledTransactionMessageCodec().encode(compiledMessage);
    
    const newTransaction = {
      ...originalTransaction,
      messageBytes: encodedMessage as TransactionMessageBytes
    };

    const base64 = getBase64EncodedWireTransaction(newTransaction);
    return {
      transaction: base64,
      payment_amount: fee.fee_in_token,
      payment_token: fee_token,
    };

    // then sign the transaction
  }

}