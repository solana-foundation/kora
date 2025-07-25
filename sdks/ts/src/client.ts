import {
  Config,
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
  AuthenticationHeaders,
  KoraClientOptions,
} from './types/index.js';
import crypto from 'crypto';

export class KoraClient {
  private rpcUrl: string;
  private apiKey?: string;
  private hmacSecret?: string;

  constructor({rpcUrl, apiKey, hmacSecret}: KoraClientOptions) {
    this.rpcUrl = rpcUrl;
    this.apiKey = apiKey;
    this.hmacSecret = hmacSecret;
  }

  private getHmacSignature({timestamp, body}: {timestamp: string, body: string}): string {
    if (!this.hmacSecret) {
      throw new Error('HMAC secret is not set');
    }
    const message = timestamp + body;
    return crypto.createHmac('sha256', this.hmacSecret).update(message).digest('hex');
  }

  private getHeaders({body}: {body: string}): AuthenticationHeaders {
    const headers: AuthenticationHeaders = {};
    if (this.apiKey) {
      headers['x-api-key'] = this.apiKey;
    }
    if (this.hmacSecret) {
      const timestamp = Math.floor(Date.now() / 1000).toString();
      const signature = this.getHmacSignature({timestamp, body});
      headers['x-timestamp'] = timestamp;
      headers['x-hmac-signature'] = signature;
    }
    return headers;
  }

  private async rpcRequest<T>(method: string, params: any): Promise<T> {
    const body = JSON.stringify({
      jsonrpc: '2.0',
      id: 1,
      method,
      params,
    });
    const headers = this.getHeaders({body});
    const response = await fetch(this.rpcUrl, {
      method: 'POST',
      headers: { ...headers, 'Content-Type': 'application/json' },
      body,
    });

    const json = await response.json() as { error?: RpcError; result: T };
    
    if ('error' in json) {
      const error = json.error!;
      throw new Error(`RPC Error ${error.code}: ${error.message}`);
    }

    return json.result;
  }

  async getConfig(): Promise<Config> {
    return this.rpcRequest<Config>('getConfig', []);
  }

  async getBlockhash(): Promise<GetBlockhashResponse> {
    return this.rpcRequest<GetBlockhashResponse>('getBlockhash', []);
  }

  async getSupportedTokens(): Promise<GetSupportedTokensResponse> {
    return this.rpcRequest<GetSupportedTokensResponse>('getSupportedTokens', []);
  }

  async estimateTransactionFee(transaction: string, feeToken: string): Promise<{ fee_in_lamports: number }> {
    return this.rpcRequest<{ fee_in_lamports: number }>('estimateTransactionFee', [transaction, feeToken]);
  }

  async signTransaction(request: SignTransactionRequest): Promise<SignTransactionResponse> {
    return this.rpcRequest<SignTransactionResponse>('signTransaction', request);
  }

  async signAndSendTransaction(
    request: SignAndSendTransactionRequest
  ): Promise<SignAndSendTransactionResponse> {
    return this.rpcRequest<SignAndSendTransactionResponse>('signAndSendTransaction', request);
  }

  async signTransactionIfPaid(
    request: SignTransactionIfPaidRequest
  ): Promise<SignTransactionIfPaidResponse> {
    return this.rpcRequest<SignTransactionIfPaidResponse>('signTransactionIfPaid', request);
  }

  async transferTransaction(
    request: TransferTransactionRequest
  ): Promise<TransferTransactionResponse> {
    return this.rpcRequest<TransferTransactionResponse>('transferTransaction', request);
  }
}