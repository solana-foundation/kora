import {
    type Blockhash,
    type Address,
    type Lamports,
    type Base64EncodedWireTransaction,
    type Signature,
    type Base58EncodedDataResponse,
} from "@solana/kit";

type ValidationConfig = Readonly<{
    max_allowed_lamports: number;
    max_signatures: number;
    price_source: 'Jupiter' | 'Mock';
    allowed_programs: Address[];
    allowed_tokens: Address[];
    allowed_spl_paid_tokens: Address[];
    disallowed_accounts: Address[];
    fee_payer_policy: FeePayerPolicy;
}>;

type FeePayerPolicy = Readonly<{
    allow_sol_transfers: boolean;
    allow_spl_transfers: boolean;
    allow_token2022_transfers: boolean;
    allow_assign: boolean;
}>;

type GetBlockHashResponse = {
    blockhash: Blockhash;
}

type GetConfigResponse = Readonly<{
    fee_payer: Address;
    validation_config: ValidationConfig;
}>;

type GetSupportedTokensResponse = Readonly<{
    tokens: Address[];
}>;

type EstimateTransactionFeeParams = Readonly<{
    transaction: Base64EncodedWireTransaction;
    fee_token: Address;
}>;

type EstimateTransactionFeeResponse = Readonly<{
    fee_in_lamports: Lamports;
}>;

type SignTransactionParams = Readonly<{
    transaction: Base64EncodedWireTransaction;
}>;

type SignTransactionResponse = Readonly<{
    signature: Signature;
    signed_transaction: Base64EncodedWireTransaction;
}>;

type SignAndSendTransactionParams = Readonly<{
    transaction: Base64EncodedWireTransaction;
}>;

type SignAndSendTransactionResponse = Readonly<{
    signature: Signature;
    signed_transaction: Base64EncodedWireTransaction;
}>;

type TransferTransactionParams = Readonly<{
    amount: number;
    token: Address;
    source: Address;
    destination: Address;
}>;

type TransferTransactionResponse = Readonly<{
    transaction: Base64EncodedWireTransaction;
    message: Base58EncodedDataResponse ;
    blockhash: Blockhash;
}>;

type SignTransactionIfPaidParams = Readonly<{
    transaction: Base64EncodedWireTransaction;
    margin: number; 
}>;

type SignTransactionIfPaidResponse = Readonly<{
    signature: Signature;
    signed_transaction: Base64EncodedWireTransaction;
}>;

type JsonRpcResponse<T> = {
    jsonrpc: string;
    id: number;
    result: T;
}

type KoraClient = {
    estimateTransactionFee(params: EstimateTransactionFeeParams): JsonRpcResponse<EstimateTransactionFeeResponse>;
    getBlockhash(): JsonRpcResponse<GetBlockHashResponse>;
    getConfig(): JsonRpcResponse<GetConfigResponse>;
    getSupportedTokens(): JsonRpcResponse<GetSupportedTokensResponse>;
    signTransaction(params: SignTransactionParams): JsonRpcResponse<SignTransactionResponse>;
    transferTransaction(params: TransferTransactionParams): JsonRpcResponse<TransferTransactionResponse>;
    signAndSendTransaction(params: SignAndSendTransactionParams): JsonRpcResponse<SignAndSendTransactionResponse>;
    signTransactionIfPaid(params: SignTransactionIfPaidParams): JsonRpcResponse<SignTransactionIfPaidResponse>;
}

export type { KoraClient };