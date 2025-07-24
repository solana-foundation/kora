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
} from './types/index.js';

export class KoraClient {
  private rpcUrl: string;

  constructor(rpcUrl: string) {
    this.rpcUrl = rpcUrl;
  }

  private async rpcRequest<T>(method: string, params: any): Promise<T> {
    const response = await fetch(this.rpcUrl, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
      },
      body: JSON.stringify({
        jsonrpc: '2.0',
        id: 1,
        method,
        params,
      }),
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