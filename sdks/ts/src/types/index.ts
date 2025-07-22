// Request Types
export interface TransferTransactionRequest {
  amount: number;
  token: string;
  source: string;
  destination: string;
}

export interface SignTransactionRequest {
  transaction: string;
}

export interface SignAndSendTransactionRequest {
  transaction: string;
}

export interface SignTransactionIfPaidRequest {
  transaction: string;
  margin?: number;
}

// Response Types
export interface TransferTransactionResponse {
  transaction: string;
  message: string;
  blockhash: string;
}

export interface SignTransactionResponse {
  signature: string;
  signed_transaction: string;
}

export interface SignAndSendTransactionResponse {
  signature: string;
  signed_transaction: string;
}

export interface SignTransactionIfPaidResponse {
  transaction: string;
  signed_transaction: string;
}

export interface GetBlockhashResponse {
  blockhash: string;
}

export interface GetSupportedTokensResponse {
  tokens: string[];
}

// Configuration Types
export interface TokenPriceInfo {
  price: number;
}

export interface ValidationConfig {
  max_allowed_lamports: number;
  max_signatures: number;
  price_source: 'Jupiter' | 'Mock';
  allowed_programs: string[];
  allowed_tokens: string[];
  allowed_spl_paid_tokens: string[];
  disallowed_accounts: string[];
  fee_payer_policy: FeePayerPolicy;
}

export interface Config {
  fee_payer: string;
  validation_config: ValidationConfig;
}

// Error Types
export interface RpcError {
  code: number;
  message: string;
} 

export interface FeePayerPolicy {
  allow_sol_transfers: boolean;
  allow_spl_transfers: boolean;
  allow_token2022_transfers: boolean;
  allow_assign: boolean;
}