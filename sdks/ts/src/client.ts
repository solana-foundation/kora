import { assertIsAddress, createNoopSigner, Instruction } from '@solana/kit';
import {
    Config,
    EstimateTransactionFeeRequest,
    EstimateTransactionFeeResponse,
    GetBlockhashResponse,
    GetSupportedTokensResponse,
    SignAndSendTransactionRequest,
    SignAndSendTransactionResponse,
    SignTransactionRequest,
    SignTransactionResponse,
    TransferTransactionRequest,
    TransferTransactionResponse,
    RpcError,
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
 * @example
 * ```typescript
 * const client = new KoraClient({ rpcUrl: 'http://localhost:8080' });
 * const config = await client.getConfig();
 * ```
 */
export class KoraClient {
    private rpcUrl: string;
    private apiKey?: string;
    private hmacSecret?: string;

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
            body,
        });

        const json = (await response.json()) as { error?: RpcError; result: T };

        if (json.error) {
            const error = json.error!;
            throw new Error(`RPC Error ${error.code}: ${error.message}`);
        }

        return json.result;
    }

    async getConfig(): Promise<Config> {
        return this.rpcRequest<Config, undefined>('getConfig', undefined);
    }

    async getPayerSigner(): Promise<GetPayerSignerResponse> {
        return this.rpcRequest<GetPayerSignerResponse, undefined>('getPayerSigner', undefined);
    }

    async getBlockhash(): Promise<GetBlockhashResponse> {
        return this.rpcRequest<GetBlockhashResponse, undefined>('getBlockhash', undefined);
    }

    async getSupportedTokens(): Promise<GetSupportedTokensResponse> {
        return this.rpcRequest<GetSupportedTokensResponse, undefined>('getSupportedTokens', undefined);
    }

    async estimateTransactionFee(request: EstimateTransactionFeeRequest): Promise<EstimateTransactionFeeResponse> {
        return this.rpcRequest<EstimateTransactionFeeResponse, EstimateTransactionFeeRequest>(
            'estimateTransactionFee',
            request,
        );
    }

    async signTransaction(request: SignTransactionRequest): Promise<SignTransactionResponse> {
        return this.rpcRequest<SignTransactionResponse, SignTransactionRequest>('signTransaction', request);
    }

    async signAndSendTransaction(request: SignAndSendTransactionRequest): Promise<SignAndSendTransactionResponse> {
        return this.rpcRequest<SignAndSendTransactionResponse, SignAndSendTransactionRequest>(
            'signAndSendTransaction',
            request,
        );
    }

    async transferTransaction(request: TransferTransactionRequest): Promise<TransferTransactionResponse> {
        const response = await this.rpcRequest<TransferTransactionResponse, TransferTransactionRequest>(
            'transferTransaction',
            request,
        );

        response.instructions = getInstructionsFromBase64Message(response.message || '');

        return response;
    }

    /**
     * Estimates the fee and builds a payment transfer instruction from the source wallet
     * to the Kora payment address. The server handles decimal conversion internally.
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
        signer_key,
        sig_verify,
    }: GetPaymentInstructionRequest): Promise<GetPaymentInstructionResponse> {
        assertIsAddress(source_wallet);
        assertIsAddress(fee_token);
        assertIsAddress(token_program_id);

        const { fee_in_token, payment_address, signer_pubkey } = await this.estimateTransactionFee({
            transaction,
            fee_token,
            sig_verify,
            signer_key,
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

        const signer = createNoopSigner(source_wallet);

        const paymentInstruction: Instruction = getTransferInstruction({
            source: sourceTokenAccount,
            destination: destinationTokenAccount,
            authority: signer,
            amount: fee_in_token,
        });

        return {
            original_transaction: transaction,
            payment_instruction: paymentInstruction,
            payment_amount: fee_in_token,
            payment_token: fee_token,
            payment_address,
            signer_address: signer_pubkey,
            signer,
        };
    }
}
