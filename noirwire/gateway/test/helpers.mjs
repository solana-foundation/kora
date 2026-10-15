import {
  AddressLookupTableAccount,
  Keypair,
  PublicKey,
  SystemProgram,
  TransactionInstruction,
  TransactionMessage,
  VersionedTransaction,
} from "@solana/web3.js";
import { Sessions } from "../src/auth.mjs";
import { Challenges } from "../src/challenges.mjs";
import { MemoryStore } from "../src/memory-store.mjs";
import { usdcAccountOf } from "../src/payments.mjs";
import { PYTH_SOL_USD } from "../src/price.mjs";
import { QuoteBook } from "../src/quotes.mjs";

export const BLOCKHASH = Keypair.generate().publicKey.toBase58();
export const U64_MAX = 2n ** 64n - 1n;
export const TOKEN_PROGRAM = new PublicKey("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA");
export const PYTH_RECEIVER = new PublicKey("rec5EKMGg6MxZYaMdyBfgwp4d5rB9T1VQH5pJv5LtFJ");
export const MICRO_USDC_PER_SOL = 150_000_000n;

export const wallet = () => Keypair.generate();

/** An instruction of some other program that is handed these accounts. */
export const programCall = (programId, accounts, data = Buffer.from([1, 2, 3])) =>
  new TransactionInstruction({
    programId,
    keys: accounts.map(({ pubkey, isSigner = false, isWritable = true }) => ({ pubkey, isSigner, isWritable })),
    data,
  });

export function buildMessage({ payer, instructions, tables = [], legacy = false }) {
  const message = new TransactionMessage({ payerKey: payer, recentBlockhash: BLOCKHASH, instructions });
  return legacy ? message.compileToLegacyMessage() : message.compileToV0Message(tables);
}

export const buildTransaction = (options) => new VersionedTransaction(buildMessage(options));

export const base64Of = (tx) => Buffer.from(tx.serialize()).toString("base64");

export function lookupTable(addresses, { active = true } = {}) {
  return new AddressLookupTableAccount({
    key: Keypair.generate().publicKey,
    state: {
      deactivationSlot: active ? U64_MAX : 123n,
      lastExtendedSlot: 0,
      lastExtendedSlotStartIndex: 0,
      authority: undefined,
      addresses,
    },
  });
}

export function tokenAccountData({ mint, owner, amount = 0n }) {
  const data = Buffer.alloc(165);
  mint.toBuffer().copy(data, 0);
  owner.toBuffer().copy(data, 32);
  data.writeBigUInt64LE(amount, 64);
  return data;
}

export const accountInfo = (owner, data = Buffer.alloc(0)) => ({ owner, data, lamports: 1_000_000, executable: false });

/** A recent, fully verified, narrow-interval Pyth SOL/USD update at `micro` micro-dollars per SOL. */
export function pythAccount({ publishTime, price = 150_00000000n, confidence = 10000000n, verification = 1, owner = PYTH_RECEIVER }) {
  const data = Buffer.alloc(134);
  data[40] = verification;
  Buffer.from("ef0d8b6fda2ceba41da15d4095d1da392a0d2f8ed0c6c7bc0f4cfac8c280b56d", "hex").copy(data, 41);
  data.writeBigInt64LE(price, 73);
  data.writeBigUInt64LE(confidence, 81);
  data.writeInt32LE(-8, 89);
  data.writeBigInt64LE(BigInt(publishTime), 93);
  return accountInfo(owner, data);
}

export { PYTH_SOL_USD };

/** A chain that answers from memory. Override fields per test. */
export class FakeChain {
  constructor() {
    this.accounts = new Map();
    this.tables = new Map();
    this.balance = 1_000_000_000n;
    this.loss = 0n;
    this.simulationError = null;
    this.blockhashValid = true;
    this.statuses = new Map();
    this.feePayerLoss = 5000n;
    this.sent = [];
    this.sendError = null;
    this.rpcCalls = 0;
  }

  setAccount(key, info) {
    this.accounts.set(key.toBase58(), info);
  }

  async getAccountInfo(key) {
    this.rpcCalls += 1;
    return this.accounts.get(key.toBase58()) ?? null;
  }

  async getMultipleAccountsInfo(keys) {
    this.rpcCalls += 1;
    return keys.map((key) => this.accounts.get(key.toBase58()) ?? null);
  }

  async getAddressLookupTable(key) {
    this.rpcCalls += 1;
    return this.tables.get(key.toBase58()) ?? null;
  }

  async getBalance() {
    this.rpcCalls += 1;
    return this.balance;
  }

  async simulate() {
    this.rpcCalls += 1;
    return { err: this.simulationError, feePayerLamports: this.balance - this.loss };
  }

  async isBlockhashValid() {
    this.rpcCalls += 1;
    return this.blockhashValid;
  }

  async sendRawTransaction(raw) {
    this.rpcCalls += 1;
    if (this.sendError) throw this.sendError;
    this.sent.push(raw);
    return "sig";
  }

  async getSignatureStatus(signature) {
    return this.statuses.get(signature) ?? null;
  }

  async getFeePayerLoss() {
    return this.feePayerLoss;
  }
}

export function testConfig(overrides = {}) {
  const feePayer = overrides.feePayerKeypair ?? Keypair.generate();
  return {
    network: "devnet",
    charge: true,
    port: 0,
    rpcUrl: "http://rpc.invalid",
    koraUrl: "http://kora.invalid",
    koraApiKey: "api-key",
    koraHmacSecret: "hmac-secret",
    databaseUrl: "postgres://invalid",
    trustedProxyHops: 0,
    quoteTtlSeconds: 60,
    abuse: {
      walletDailyLossLamports: 50_000n,
      ipDailyLossLamports: 200_000n,
      appDailyLossLamports: 100_000n,
      failureBlockMinSends: 5,
      failureBlockRatioPpm: 200_000,
    },
    apps: { dailyCapDefaultLamports: 10_000_000n, dailyCapMaxLamports: 500_000_000n },
    reconciliationStaleLimit: 20,
    feePayerThresholds: { lowLamports: 30_000_000n, emptyLamports: 5_000_000n },
    dailyLossCeilingLamports: 100_000n,
    caps: {
      maxSignatures: 3,
      maxTxBytes: 1232,
      maxComputeUnitPrice: 50_000,
      maxPriorityFeeLamports: 10_000,
      maxComputeUnitLimit: 1_400_000,
      maxFeePayerOutflowLamports: 2_200_000,
    },
    limits: { globalPerMinute: 100_000, ipPerMinute: 1000, userPerMinute: 1000, registerPerMinute: 1000 },
    feePayer: feePayer.publicKey,
    usdcMint: Keypair.generate().publicKey,
    paymentUsdcAccount: Keypair.generate().publicKey,
    ...overrides,
    feePayerKeypair: undefined,
  };
}

/** A fake Kora that really signs as the fee payer, and records what it was asked. */
export function fakeKora(feePayerKeypair) {
  const kora = {
    calls: [],
    behaviour: null,
    async signTransaction(base64) {
      kora.calls.push(base64);
      if (kora.behaviour) throw kora.behaviour;
      const tx = VersionedTransaction.deserialize(Buffer.from(base64, "base64"));
      tx.sign([feePayerKeypair]);
      return Buffer.from(tx.serialize()).toString("base64");
    },
  };
  return kora;
}

export function makeContext(overrides = {}) {
  const feePayerKeypair = Keypair.generate();
  const config = testConfig({ feePayerKeypair, ...overrides.config });
  const clock = { ms: 1_800_000_000_000 };
  const chain = new FakeChain();
  const ctx = {
    config,
    chain,
    store: new MemoryStore({ now: () => clock.ms }),
    kora: fakeKora(feePayerKeypair),
    quotes: new QuoteBook({ now: () => clock.ms }),
    challenges: new Challenges({ now: () => clock.ms }),
    sessions: new Sessions({ now: () => clock.ms }),
    readPrice: async () => MICRO_USDC_PER_SOL,
    now: () => clock.ms,
    logger: false,
    ...overrides.ctx,
  };
  return { ctx, clock, chain, feePayerKeypair, config };
}

/** A user with a funded USDC account, as the chain sees it. */
export function fundedUser(chain, config, amount = 50_000_000n) {
  const user = Keypair.generate();
  chain.setAccount(usdcAccountOf(config.usdcMint, user.publicKey), accountInfo(TOKEN_PROGRAM, tokenAccountData({ mint: config.usdcMint, owner: user.publicKey, amount })));
  return user;
}

/** An app in the store with a payout wallet set, unless overridden. */
export async function seedApp(ctx, overrides = {}) {
  const app = {
    id: `app_${Math.random().toString(36).slice(2, 12).padEnd(10, "0")}`,
    owner: Keypair.generate().publicKey.toBase58(),
    name: "Acme",
    payoutWallet: Keypair.generate().publicKey.toBase58(),
    markupBps: 10_000,
    allowedOrigins: [],
    allowedPrograms: [],
    dailyCapLamports: ctx.config.apps.dailyCapDefaultLamports,
    paused: false,
    ...overrides,
  };
  await ctx.store.createApp(app);
  return app;
}

/** A row in the ledger, admitted with budgets nothing can exceed, then moved to `status`. */
let seeded = 0;
export async function seedRow(store, { appId, user, ip = "1.1.1.1", status = "landed", loss = 0n, sponsored = 0n, networkCost = 1n, markup = 1n, app = 0n }) {
  seeded += 1;
  const messageHash = `seed-${seeded}`;
  const huge = 10n ** 18n;
  await store.reserve(
    { messageHash, appId, userWallet: user, clientIp: ip, blockhash: "b", networkCostLamports: 1n, networkCost, markup, noirwire: markup - app, app, lossLamports: loss, sponsoredLamports: sponsored },
    { sinceMs: 0, globalLoss: huge, walletLoss: huge, ipLoss: huge, appLoss: huge, appCap: huge },
  );
  await store.recordStatus(messageHash, { status });
  return messageHash;
}

export const transferInstruction = (from, to, lamports = 1) => SystemProgram.transfer({ fromPubkey: from, toPubkey: to, lamports });
