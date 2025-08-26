// TODO Make sure to change necessary deps from devdeps to deps
import { assertIsAddress, createNoopSigner, Instruction } from '@solana/kit';
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
    GetPayerSignerResponse,
    GetPaymentInstructionRequest,
    GetPaymentInstructionResponse,
} from './types/index.js';
import crypto from 'crypto';
import { getInstructionsFromBase64Message } from './utils/transaction.js';
import { findAssociatedTokenPda, TOKEN_PROGRAM_ADDRESS, getTransferInstruction } from '@solana-program/token';

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

    private getHmacSignature({ timestamp, body }: { timestamp: string; body: string }): string {
        if (!this.hmacSecret) {
            throw new Error('HMAC secret is not set');
        }
        const message = timestamp + body;
        return crypto.createHmac('sha256', this.hmacSecret).update(message).digest('hex');
    }

    private getHeaders({ body }: { body: string }): AuthenticationHeaders {
        const headers: AuthenticationHeaders = {};
        if (this.apiKey) {
            headers['x-api-key'] = this.apiKey;
        }
        if (this.hmacSecret) {
            const timestamp = Math.floor(Date.now() / 1000).toString();
            const signature = this.getHmacSignature({ timestamp, body });
            headers['x-timestamp'] = timestamp;
            headers['x-hmac-signature'] = signature;
        }
        return headers;
    }

    private async rpcRequest<T, U>(method: string, params: U): Promise<T> {
        const body = JSON.stringify({
            jsonrpc: '2.0',
            id: 1,
            method,
            params,
        });
        const headers = this.getHeaders({ body });
        const response = await fetch(this.rpcUrl, {
            method: 'POST',
            headers: { ...headers, 'Content-Type': 'application/json' },
            body: JSON.stringify({
                jsonrpc: '2.0',
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
        return this.rpcRequest<Config, undefined>('getConfig', undefined);
    }

    /**
     * Retrieves the payer signer and payment destination from the Kora server.
     * @returns Object containing the payer signer and payment destination
     * @throws {Error} When the RPC call fails
     *
     * @example
     */
    async getPayerSigner(): Promise<GetPayerSignerResponse> {
        return this.rpcRequest<GetPayerSignerResponse, undefined>('getPayerSigner', undefined);
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
        return this.rpcRequest<GetBlockhashResponse, undefined>('getBlockhash', undefined);
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
        return this.rpcRequest<GetSupportedTokensResponse, undefined>('getSupportedTokens', undefined);
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
    async estimateTransactionFee(request: EstimateTransactionFeeRequest): Promise<EstimateTransactionFeeResponse> {
        return this.rpcRequest<EstimateTransactionFeeResponse, EstimateTransactionFeeRequest>(
            'estimateTransactionFee',
            request,
        );
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
    async signTransaction(request: SignTransactionRequest): Promise<SignTransactionResponse> {
        return this.rpcRequest<SignTransactionResponse, SignTransactionRequest>('signTransaction', request);
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
    async signAndSendTransaction(request: SignAndSendTransactionRequest): Promise<SignAndSendTransactionResponse> {
        return this.rpcRequest<SignAndSendTransactionResponse, SignAndSendTransactionRequest>(
            'signAndSendTransaction',
            request,
        );
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
    async signTransactionIfPaid(request: SignTransactionIfPaidRequest): Promise<SignTransactionIfPaidResponse> {
        return this.rpcRequest<SignTransactionIfPaidResponse, SignTransactionIfPaidRequest>(
            'signTransactionIfPaid',
            request,
        );
    }

    /**
     * Creates a token transfer transaction with Kora as the fee payer.
     * @param request - Transfer request parameters
     * @param request.amount - Amount to transfer (in token's smallest unit)
     * @param request.token - Mint address of the token to transfer
     * @param request.source - Source wallet public key
     * @param request.destination - Destination wallet public key
     * @returns Base64-encoded signed transaction, base64-encoded message, blockhash, and parsed instructions
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
     * console.log('Instructions:', transfer.instructions);
     * ```
     */
    async transferTransaction(request: TransferTransactionRequest): Promise<TransferTransactionResponse> {
        const response = await this.rpcRequest<TransferTransactionResponse, TransferTransactionRequest>(
            'transferTransaction',
            request,
        );

        // Parse instructions from the message to enhance developer experience
        // Always set instructions, even for empty messages (for consistency)
        response.instructions = getInstructionsFromBase64Message(response.message || '');

        return response;
    }

    /**
     * Creates a payment instruction to append to a transaction for fee payment to the Kora paymaster.
     *
     * This method estimates the required fee and generates a token transfer instruction
     * from the source wallet to the Kora payment address. The server handles decimal
     * conversion internally, so the raw token amount is used directly.
     *
     * @param request - Payment instruction request parameters
     * @param request.transaction - Base64-encoded transaction to estimate fees for
     * @param request.fee_token - Mint address of the token to use for payment
     * @param request.source_wallet - Public key of the wallet paying the fees
     * @param request.token_program_id - Optional token program ID (defaults to TOKEN_PROGRAM_ADDRESS)
     * @returns Payment instruction details including the instruction, amount, and addresses
     * @throws {Error} When the token is not supported, payment is not required, or invalid addresses are provided
     *
     * @example
     * ```typescript
     * const paymentInfo = await client.getPaymentInstruction({
     *   transaction: 'base64EncodedTransaction',
     *   fee_token: 'EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v',
     *   source_wallet: 'sourceWalletPublicKey'
     * });
     * // Append paymentInfo.payment_instruction to your transaction
     * ```
     */
    async getPaymentInstruction({
        transaction,
        fee_token,
        source_wallet,
        token_program_id = TOKEN_PROGRAM_ADDRESS,
    }: GetPaymentInstructionRequest): Promise<GetPaymentInstructionResponse> {
        assertIsAddress(source_wallet);
        assertIsAddress(fee_token);
        assertIsAddress(token_program_id);

        const { fee_in_token, payment_address, signer_pubkey } = await this.estimateTransactionFee({
            transaction,
            fee_token,
        });
        assertIsAddress(payment_address);

        const [sourceTokenAccount] = await findAssociatedTokenPda({
            owner: source_wallet,
            tokenProgram: token_program_id,
            mint: fee_token,
        });

        const [destinationTokenAccount] = await findAssociatedTokenPda({
            owner: payment_address,
            tokenProgram: token_program_id,
            mint: fee_token,
        });

        const paymentInstruction: Instruction = getTransferInstruction({
            source: sourceTokenAccount,
            destination: destinationTokenAccount,
            authority: createNoopSigner(source_wallet),
            amount: fee_in_token,
        });

        return {
            original_transaction: transaction,
            payment_instruction: paymentInstruction,
            payment_amount: fee_in_token,
            payment_token: fee_token,
            payment_address,
            signer_address: signer_pubkey,
        };
    }
}
