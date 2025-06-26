import { Config, GetBlockhashResponse, GetSupportedTokensResponse, SignAndSendTransactionRequest, SignAndSendTransactionResponse, SignTransactionIfPaidRequest, SignTransactionIfPaidResponse, SignTransactionRequest, SignTransactionResponse, TransferTransactionRequest, TransferTransactionResponse } from './types/index.js';
export declare class KoraClient {
    private rpcUrl;
    constructor(rpcUrl: string);
    private rpcRequest;
    getConfig(): Promise<Config>;
    getBlockhash(): Promise<GetBlockhashResponse>;
    getSupportedTokens(): Promise<GetSupportedTokensResponse>;
    estimateTransactionFee(transaction: string, feeToken: string): Promise<{
        fee_in_lamports: number;
    }>;
    signTransaction(request: SignTransactionRequest): Promise<SignTransactionResponse>;
    signAndSendTransaction(request: SignAndSendTransactionRequest): Promise<SignAndSendTransactionResponse>;
    signTransactionIfPaid(request: SignTransactionIfPaidRequest): Promise<SignTransactionIfPaidResponse>;
    transferTransaction(request: TransferTransactionRequest): Promise<TransferTransactionResponse>;
}
