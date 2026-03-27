import { Address, assertIsAddress, Instruction, isTransactionSigner } from '@solana/kit';
import { findAssociatedTokenPda, getTransferInstruction, TOKEN_PROGRAM_ADDRESS } from '@solana-program/token';
import crypto from 'crypto';

import {
    AuthenticationHeaders,
    Config,
    EstimateTransactionFeeRequest,
    EstimateTransactionFeeResponse,
    GetBlockhashResponse,
    GetPayerSignerResponse,
    GetPaymentInstructionRequest,
    GetPaymentInstructionResponse,
    GetSupportedTokensResponse,
    KoraClientOptions,
    RpcError,
    SignAndSendTransactionRequest,
    SignAndSendTransactionResponse,
    SignTransactionRequest,
    SignTransactionResponse,
    TransferTransactionRequest,
    TransferTransactionResponse,
} from './types/index.js';
import { getInstructionsFromBase64Message } from './utils/transaction.js';

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

    private getHmacSignature({ timestamp, body }: { body: string; timestamp: string }): string {
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
            const signature = this.getHmacSignature({ body, timestamp });
            headers['x-timestamp'] = timestamp;
            headers['x-hmac-signature'] = signature;
        }
        return headers;
    }

    private async rpcRequest<T, U>(method: string, params: U): Promise<T> {
        const body = JSON.stringify({
            id: 1,
            jsonrpc: '2.0',
            method,
            params,
        });
        const headers = this.getHeaders({ body });
        const response = await fetch(this.rpcUrl, {
            body,
            headers: { ...headers, 'Content-Type': 'application/json' },
            method: 'POST',
        });

        const json = (await response.json()) as { error?: RpcError; result: T };

        if (json.error) {
            const error = json.error;
            throw new Error(`RPC Error ${error.code}: ${error.message}`);
        }

        return json.result;
    }

    async getConfig(): Promise<Config> {
        return await this.rpcRequest<Config, undefined>('getConfig', undefined);
    }

    async getPayerSigner(): Promise<GetPayerSignerResponse> {
        return await this.rpcRequest<GetPayerSignerResponse, undefined>('getPayerSigner', undefined);
    }

    async getBlockhash(): Promise<GetBlockhashResponse> {
        return await this.rpcRequest<GetBlockhashResponse, undefined>('getBlockhash', undefined);
    }

    async getSupportedTokens(): Promise<GetSupportedTokensResponse> {
        return await this.rpcRequest<GetSupportedTokensResponse, undefined>('getSupportedTokens', undefined);
    }

    async estimateTransactionFee(request: EstimateTransactionFeeRequest): Promise<EstimateTransactionFeeResponse> {
        return await this.rpcRequest<EstimateTransactionFeeResponse, EstimateTransactionFeeRequest>(
            'estimateTransactionFee',
            request,
        );
    }

    async signTransaction(request: SignTransactionRequest): Promise<SignTransactionResponse> {
        return await this.rpcRequest<SignTransactionResponse, SignTransactionRequest>('signTransaction', request);
    }

    async signAndSendTransaction(request: SignAndSendTransactionRequest): Promise<SignAndSendTransactionResponse> {
        return await this.rpcRequest<SignAndSendTransactionResponse, SignAndSendTransactionRequest>(
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
        const sourceIsSigner = typeof source_wallet !== 'string' && isTransactionSigner(source_wallet);
        const walletAddress: Address = sourceIsSigner ? source_wallet.address : (source_wallet as Address);

        assertIsAddress(walletAddress);
        assertIsAddress(fee_token);
        assertIsAddress(token_program_id);

        const { fee_in_token, payment_address, signer_pubkey } = await this.estimateTransactionFee({
            fee_token,
            sig_verify,
            signer_key,
            transaction,
        });
        assertIsAddress(payment_address);

        const [sourceTokenAccount] = await findAssociatedTokenPda({
            mint: fee_token,
            owner: walletAddress,
            tokenProgram: token_program_id,
        });

        const [destinationTokenAccount] = await findAssociatedTokenPda({
            mint: fee_token,
            owner: payment_address,
            tokenProgram: token_program_id,
        });

        const paymentInstruction: Instruction = getTransferInstruction({
            amount: fee_in_token,
            authority: sourceIsSigner ? source_wallet : walletAddress,
            destination: destinationTokenAccount,
            source: sourceTokenAccount,
        });

        return {
            original_transaction: transaction,
            payment_address,
            payment_amount: fee_in_token,
            payment_instruction: paymentInstruction,
            payment_token: fee_token,
            signer_address: signer_pubkey,
        };
    }
}
