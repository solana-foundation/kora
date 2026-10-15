import { PublicKey, Transaction, VersionedTransaction } from "@solana/web3.js";

export type Network = "devnet" | "mainnet";
export type AnyTransaction = Transaction | VersionedTransaction;

/** Where each network's relayer answers. Pass `url` to use another deployment. */
export const NETWORK_URLS: Record<Network, string> = {
  devnet: "https://devnet.relayer.noirwire.com",
  mainnet: "https://relayer.noirwire.com",
};

export interface RelayerOptions {
  /** The app id from the console. */
  app: string;
  /** Which public deployment to call. Ignored when `url` is given. */
  network?: Network;
  /** The base URL of a relayer deployment. */
  url?: string;
  /** For tests and non-browser runtimes; defaults to the global `fetch`. */
  fetch?: typeof fetch;
}

/** What a user wallet must offer: its address and a way to sign a transaction. */
export interface Wallet {
  publicKey: PublicKey;
  signTransaction<T extends AnyTransaction>(transaction: T): Promise<T>;
}

/** Amounts are integers in micro-USDC (6 decimals), as strings. */
export interface Quote {
  networkCostMicroUsdc: string;
  markupMicroUsdc: string;
  appMicroUsdc: string;
  noirwireMicroUsdc: string;
  totalMicroUsdc: string;
}

export interface Prepared<T extends AnyTransaction> {
  /** The transaction with the fee payer set and the payment appended. The user signs this one. */
  transaction: T;
  quote: Quote;
  expiresAt: string;
}

export interface RelayerConfig {
  network: Network;
  feePayer: string;
  usdcMint: string;
  charge: boolean;
  markup: { defaultBps: number; maxBps: number };
  split: { appBps: number; noirwireBps: number };
}

/** The relayer accepted the transaction for the network. It may not have landed yet. */
export interface SentResult {
  outcome: "sent";
  signature: string;
  status: string;
}

/**
 * The relayer could not say whether it signed or sent the transaction. It may still land. It
 * was not retried and must not be sent again by anyone with another fee payer; look the
 * `signature` (when present) up on chain, or ask for the same message again.
 */
export interface UnknownResult {
  outcome: "unknown";
  messageHash: string;
  signature?: string;
}

export type SendResult = SentResult | UnknownResult;

/**
 * The relayer refused the request and nothing was signed or sent. For `send` this is only a
 * documented refusal (a 4xx with a code, or a 503 the relayer gives before it signs); any other
 * answer to a send is a `SendOutcomeUnknownError`.
 */
export class RelayerError extends Error {
  constructor(
    message: string,
    readonly code: string,
    readonly status: number,
  ) {
    super(message);
    this.name = "RelayerError";
  }
}

/**
 * A send got no usable answer: a dropped connection, a timeout, a server error or a reply that
 * is not one the relayer documents. The relayer may have signed it, so the transaction may still
 * land. Do not send it again with another fee payer.
 */
export class SendOutcomeUnknownError extends Error {
  constructor(cause: unknown) {
    super(`the relayer gave no clear answer to the send, so the transaction may still land: ${cause instanceof Error ? cause.message : String(cause)}`);
    this.name = "SendOutcomeUnknownError";
  }
}

/** 503 codes the relayer returns before it calls the fee payer's signer. */
const REFUSED_BEFORE_SIGNING_503 = new Set(["loss_ceiling", "reconciliation_unhealthy", "overloaded"]);

/** Only a documented refusal, which the relayer gives before it signs, means nothing was signed or sent. */
function refusedBeforeSigning(status: number, json: any): boolean {
  const code = typeof json?.code === "string" ? json.code : null;
  if (code === null) return false;
  if (status >= 400 && status < 500) return true;
  return status === 503 && REFUSED_BEFORE_SIGNING_503.has(code);
}

const toBase64 = (bytes: Uint8Array) => Buffer.from(bytes).toString("base64");
const fromBase64 = (text: string) => Uint8Array.from(Buffer.from(text, "base64"));

function serialize(transaction: AnyTransaction): string {
  return transaction instanceof Transaction
    ? toBase64(transaction.serialize({ requireAllSignatures: false, verifySignatures: false }))
    : toBase64(transaction.serialize());
}

function deserializeLike<T extends AnyTransaction>(sample: T, base64: string): T {
  const bytes = fromBase64(base64);
  return (sample instanceof Transaction ? Transaction.from(bytes) : VersionedTransaction.deserialize(bytes)) as T;
}

export interface Relayer {
  /** The fee payer to put on your transaction. */
  feePayer(): Promise<PublicKey>;
  config(): Promise<RelayerConfig>;
  /** Validates and prices a transaction and returns it with the payment appended. */
  prepare<T extends AnyTransaction>(transaction: T, user: PublicKey): Promise<Prepared<T>>;
  /** Sends a transaction the user has signed. Never retried. */
  send(signed: AnyTransaction): Promise<SendResult>;
  /** Prepare, have the wallet sign exactly that transaction, send. Never retried. */
  signAndSend(transaction: AnyTransaction, wallet: Wallet): Promise<SendResult>;
}

export function createRelayer(options: RelayerOptions): Relayer {
  const base = (options.url ?? NETWORK_URLS[options.network ?? "devnet"]).replace(/\/+$/, "");
  const doFetch = options.fetch ?? fetch;
  let cachedConfig: Promise<RelayerConfig> | null = null;

  async function call(path: string, init?: { method: string; body: unknown }): Promise<{ status: number; json: any }> {
    const response = await doFetch(`${base}${path}`, {
      method: init?.method ?? "GET",
      headers: init ? { "content-type": "application/json" } : undefined,
      body: init ? JSON.stringify(init.body) : undefined,
    });
    const json = await response.json().catch(() => ({}));
    return { status: response.status, json };
  }

  function failure(status: number, json: any): RelayerError {
    return new RelayerError(String(json?.error ?? `the relayer answered ${status}`), String(json?.code ?? "unknown_error"), status);
  }

  const config = () => {
    cachedConfig ??= call("/v1/config").then(({ status, json }) => {
      if (status !== 200) throw failure(status, json);
      return json as RelayerConfig;
    });
    cachedConfig.catch(() => {
      cachedConfig = null;
    });
    return cachedConfig;
  };

  async function prepare<T extends AnyTransaction>(transaction: T, user: PublicKey): Promise<Prepared<T>> {
    const { status, json } = await call("/v1/prepare", {
      method: "POST",
      body: { app: options.app, transaction: serialize(transaction), user: user.toBase58() },
    });
    if (status !== 200) throw failure(status, json);
    return { transaction: deserializeLike(transaction, json.transaction), quote: json.quote, expiresAt: json.expiresAt };
  }

  async function send(signed: AnyTransaction): Promise<SendResult> {
    let answer: { status: number; json: any };
    try {
      answer = await call("/v1/send", { method: "POST", body: { transaction: serialize(signed) } });
    } catch (error) {
      throw new SendOutcomeUnknownError(error);
    }
    if (answer.status === 200 && typeof answer.json?.signature === "string") {
      return { outcome: "sent", signature: answer.json.signature, status: String(answer.json.status) };
    }
    if (answer.status === 202 && typeof answer.json?.messageHash === "string") {
      const { messageHash, signature } = answer.json;
      return { outcome: "unknown", messageHash, ...(signature ? { signature } : {}) };
    }
    if (refusedBeforeSigning(answer.status, answer.json)) throw failure(answer.status, answer.json);
    throw new SendOutcomeUnknownError(`the relayer answered ${answer.status} in a way that does not say what became of the transaction`);
  }

  return {
    async feePayer() {
      return new PublicKey((await config()).feePayer);
    },
    config,
    prepare,
    send,
    async signAndSend(transaction, wallet) {
      const prepared = await prepare(transaction, wallet.publicKey);
      const signed = await wallet.signTransaction(prepared.transaction);
      return send(signed);
    },
  };
}
