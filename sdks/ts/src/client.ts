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
} from './types/index.js';

export class KoraClient {
  private rpcUrl: string;

  constructor(rpcUrl: string) {
    this.rpcUrl = rpcUrl;
  }

  private async rpcRequest<T, U>(method: string, params: U): Promise<T> {
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
      } as RpcRequest<U>),
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

  async getBlockhash(): Promise<GetBlockhashResponse> {
    return this.rpcRequest<GetBlockhashResponse, undefined>('getBlockhash', undefined);
  }

  async getSupportedTokens(): Promise<GetSupportedTokensResponse> {
    return this.rpcRequest<GetSupportedTokensResponse, undefined>('getSupportedTokens', undefined);
  }

  async estimateTransactionFee(
    request: EstimateTransactionFeeRequest
  ): Promise<EstimateTransactionFeeResponse> {
    return this.rpcRequest<EstimateTransactionFeeResponse, EstimateTransactionFeeRequest>(
      'estimateTransactionFee',
      request
    );
  }

  async signTransaction(request: SignTransactionRequest): Promise<SignTransactionResponse> {
    return this.rpcRequest<SignTransactionResponse, SignTransactionRequest>('signTransaction', request);
  }

  async signAndSendTransaction(
    request: SignAndSendTransactionRequest
  ): Promise<SignAndSendTransactionResponse> {
    return this.rpcRequest<
      SignAndSendTransactionResponse,
      SignAndSendTransactionRequest
    >('signAndSendTransaction', request);
  }

  async signTransactionIfPaid(
    request: SignTransactionIfPaidRequest
  ): Promise<SignTransactionIfPaidResponse> {
    return this.rpcRequest<
      SignTransactionIfPaidResponse,
      SignTransactionIfPaidRequest
    >('signTransactionIfPaid', request);
  }

  async transferTransaction(
    request: TransferTransactionRequest
  ): Promise<TransferTransactionResponse> {
    return this.rpcRequest<
      TransferTransactionResponse,
      TransferTransactionRequest
    >('transferTransaction', request);
  }
}