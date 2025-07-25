import { getCreateAccountInstruction } from "@solana-program/system";
import {
  findAssociatedTokenPda,
  getCreateAssociatedTokenIdempotentInstructionAsync,
  getInitializeMintInstruction,
  getMintSize,
  getMintToInstruction,
  TOKEN_PROGRAM_ADDRESS,
} from "@solana-program/token";
import {
  airdropFactory,
  createSolanaRpc,
  createSolanaRpcSubscriptions,
  lamports,
  sendAndConfirmTransactionFactory,
  pipe,
  createTransactionMessage,
  setTransactionMessageLifetimeUsingBlockhash,
  setTransactionMessageFeePayerSigner,
  appendTransactionMessageInstructions,
  TransactionSigner,
  SolanaRpcApi,
  RpcSubscriptions,
  Rpc,
  SolanaRpcSubscriptionsApi,
  MicroLamports,
  CompilableTransactionMessage,
  TransactionMessageWithBlockhashLifetime,
  Commitment,
  Signature,
  signTransactionMessageWithSigners,
  getSignatureFromTransaction,
  Instruction,
  KeyPairSigner,
  Address,
  assertIsAddress,
  createKeyPairSignerFromBytes,
  getBase58Encoder,
} from "@solana/kit";
import {
  updateOrAppendSetComputeUnitLimitInstruction,
  updateOrAppendSetComputeUnitPriceInstruction,
  MAX_COMPUTE_UNIT_LIMIT,
  estimateComputeUnitLimitFactory,
} from "@solana-program/compute-budget";
import { config } from "dotenv";
import path from "path";
import { KoraClient } from "../src/index.js";

config({ path: path.resolve(process.cwd(), ".env") });

const DEFAULTS = {
  DECIMALS: 6,
  TOKEN_DROP_AMOUNT: 100_000,
  KORA_RPC_URL: "http://localhost:8080/",
  SOLANA_RPC_URL: "http://127.0.0.1:8899",
  SOLANA_WS_URL: "ws://127.0.0.1:8900",
  COMMITMENT: "processed" as Commitment,
  SOL_DROP_AMOUNT: 1_000_000_000,

  // DO NOT USE THESE KEYPAIRS IN PRODUCTION, TESTING KEYPAIRS ONLY
  KORA_ADDRESS: "7AqpcUvgJ7Kh1VmJZ44rWp2XDow33vswo9VK9VqpPU2d", // Make sure this matches the kora-rpc signer address on launch (root .env)
  SENDER_SECRET:
    "3Tdt5TrRGJYPbTo8zZAscNTvgRGnCLM854tCpxapggUazqdYn6VQRQ9DqNz1UkEfoPCYKj6PwSwCNtckGGvAKugb",
  TEST_USDC_MINT_SECRET:
    "59kKmXphL5UJANqpFFjtH17emEq3oRNmYsx6a3P3vSGJRmhMgVdzH77bkNEi9bArRViT45e8L2TsuPxKNFoc3Qfg", // Make sure this matches the USDC mint in kora.toml (9BgeTKqmFsPVnfYscfM6NvsgmZxei7XfdciShQ6D3bxJ)
  DESTINATION_ADDRESS: "AVmDft8deQEo78bRKcGN5ZMf3hyjeLBK4Rd4xGB46yQM",
};

interface TestSuite {
  koraClient: KoraClient;
  testWallet: KeyPairSigner<string>;
  usdcMint: Address<string>;
  destinationAddress: Address<string>;
  koraAddress: Address<string>;
}

interface Client {
  rpc: Rpc<SolanaRpcApi>;
  rpcSubscriptions: RpcSubscriptions<SolanaRpcSubscriptionsApi>;
}

const createKeyPairSignerFromB58Secret = async (b58Secret: string) => {
  const base58Encoder = getBase58Encoder();
  const b58SecretEncoded = base58Encoder.encode(b58Secret);
  return await createKeyPairSignerFromBytes(b58SecretEncoded);
};

function loadEnvironmentVariables() {
  const koraAddress = process.env.KORA_ADDRESS || DEFAULTS.KORA_ADDRESS;
  const koraRpcUrl = process.env.KORA_RPC_URL || DEFAULTS.KORA_RPC_URL;
  const solanaRpcUrl = process.env.SOLANA_RPC_URL || DEFAULTS.SOLANA_RPC_URL;
  const solanaWsUrl = process.env.SOLANA_WS_URL || DEFAULTS.SOLANA_WS_URL;
  const commitment = (process.env.COMMITMENT ||
    DEFAULTS.COMMITMENT) as Commitment;
  const tokenDecimals = Number(process.env.TOKEN_DECIMALS || DEFAULTS.DECIMALS);
  const tokenDropAmount = Number(
    process.env.TOKEN_DROP_AMOUNT || DEFAULTS.TOKEN_DROP_AMOUNT,
  );
  const solDropAmount = BigInt(
    process.env.SOL_DROP_AMOUNT || DEFAULTS.SOL_DROP_AMOUNT,
  );
  const testWalletSecret = process.env.SENDER_SECRET || DEFAULTS.SENDER_SECRET;
  const testUsdcMintSecret =
    process.env.TEST_USDC_MINT_SECRET || DEFAULTS.TEST_USDC_MINT_SECRET;
  const destinationAddress =
    process.env.DESTINATION_ADDRESS || DEFAULTS.DESTINATION_ADDRESS;
  assertIsAddress(destinationAddress);
  assertIsAddress(koraAddress);
  return {
    koraRpcUrl,
    koraAddress,
    commitment,
    tokenDecimals,
    tokenDropAmount,
    solDropAmount,
    solanaRpcUrl,
    solanaWsUrl,
    testWalletSecret,
    testUsdcMintSecret,
    destinationAddress,
  };
}

async function createKeyPairSigners() {
  const { testWalletSecret, testUsdcMintSecret, destinationAddress } =
    loadEnvironmentVariables();
  const testWallet = await createKeyPairSignerFromB58Secret(testWalletSecret);
  const usdcMint = await createKeyPairSignerFromB58Secret(testUsdcMintSecret);
  return {
    testWallet,
    usdcMint,
    destinationAddress,
  };
}

const createDefaultTransaction = async (
  client: Client,
  feePayer: TransactionSigner,
  computeLimit: number = MAX_COMPUTE_UNIT_LIMIT,
  feeMicroLamports: MicroLamports = 1n as MicroLamports,
): Promise<
  CompilableTransactionMessage & TransactionMessageWithBlockhashLifetime
> => {
  const { value: latestBlockhash } = await client.rpc
    .getLatestBlockhash()
    .send();
  return pipe(
    createTransactionMessage({ version: 0 }),
    (tx) => setTransactionMessageFeePayerSigner(feePayer, tx),
    (tx) => setTransactionMessageLifetimeUsingBlockhash(latestBlockhash, tx),
    (tx) => updateOrAppendSetComputeUnitPriceInstruction(feeMicroLamports, tx),
    (tx) => updateOrAppendSetComputeUnitLimitInstruction(computeLimit, tx),
  );
};

const signAndSendTransaction = async (
  client: Client,
  transactionMessage: CompilableTransactionMessage &
    TransactionMessageWithBlockhashLifetime,
  commitment: Commitment = loadEnvironmentVariables().commitment,
) => {
  const signedTransaction =
    await signTransactionMessageWithSigners(transactionMessage);
  const signature = getSignatureFromTransaction(signedTransaction);
  await sendAndConfirmTransactionFactory(client)(signedTransaction, {
    commitment,
  });
  return signature;
};

async function sendAndConfirmInstructions(
  client: Client,
  payer: TransactionSigner,
  instructions: Instruction[],
  description: string,
): Promise<Signature> {
  try {
    const simulationTx = await pipe(
      await createDefaultTransaction(client, payer),
      (tx) => appendTransactionMessageInstructions(instructions, tx),
    );
    const estimateCompute = estimateComputeUnitLimitFactory({
      rpc: client.rpc,
    });
    const computeUnitLimit = await estimateCompute(simulationTx);
    const signature = await pipe(
      await createDefaultTransaction(client, payer, computeUnitLimit),
      (tx) => appendTransactionMessageInstructions(instructions, tx),
      (tx) => signAndSendTransaction(client, tx),
    );
    return signature;
  } catch (error) {
    throw new Error(
      `Failed to ${description.toLowerCase()}: ${error instanceof Error ? error.message : "Unknown error"}`,
    );
  }
}

async function initializeToken({
  client,
  mintAuthority,
  payer,
  owner,
  mint,
  dropAmount,
  decimals,
  otherAtaWallets,
}: {
  client: Client;
  mintAuthority: KeyPairSigner<string>;
  payer: KeyPairSigner<string>;
  owner: KeyPairSigner<string>;
  mint: KeyPairSigner<string>;
  dropAmount: number;
  decimals: number;
  otherAtaWallets?: Address<string>[];
}) {
  // Get Owner ATA
  const [ata] = await findAssociatedTokenPda({
    mint: mint.address,
    owner: owner.address,
    tokenProgram: TOKEN_PROGRAM_ADDRESS,
  });
  // Get Mint size & rent
  const mintSpace = BigInt(getMintSize());
  const mintRent = await client.rpc
    .getMinimumBalanceForRentExemption(mintSpace)
    .send();
  // Create instructions for new token mint
  const baseInstructions = [
    // Create the Mint Account
    getCreateAccountInstruction({
      payer,
      newAccount: mint,
      lamports: mintRent,
      space: mintSpace,
      programAddress: TOKEN_PROGRAM_ADDRESS,
    }),
    // Initialize the Mint
    getInitializeMintInstruction({
      mint: mint.address,
      decimals,
      mintAuthority: mintAuthority.address,
    }),
    // Create Associated Token Account
    await getCreateAssociatedTokenIdempotentInstructionAsync({
      mint: mint.address,
      payer,
      owner: owner.address,
    }),
    // Mint To the Destination Associated Token Account
    getMintToInstruction({
      mint: mint.address,
      token: ata,
      amount: BigInt(dropAmount * 10 ** decimals),
      mintAuthority,
    }),
  ];
  // Generate Create ATA instructions for other token accounts we wish to add
  const otherAtaInstructions = otherAtaWallets
    ? await Promise.all(
        otherAtaWallets.map(
          async (wallet) =>
            await getCreateAssociatedTokenIdempotentInstructionAsync({
              mint: mint.address,
              payer,
              owner: wallet,
            }),
        ),
      )
    : [];
  const instructions = [...baseInstructions, ...otherAtaInstructions];
  await sendAndConfirmInstructions(
    client,
    payer,
    instructions,
    "Initialize token and ATAs",
  );
}

async function setupTestSuit(): Promise<TestSuite> {
  const {
    koraAddress,
    koraRpcUrl,
    commitment,
    tokenDecimals,
    tokenDropAmount,
    solDropAmount,
    solanaRpcUrl,
    solanaWsUrl,
  } = await loadEnvironmentVariables();

  // Create Solana client
  const rpc = createSolanaRpc(solanaRpcUrl);
  const rpcSubscriptions = createSolanaRpcSubscriptions(solanaWsUrl);
  const airdrop = airdropFactory({ rpc, rpcSubscriptions });
  const client: Client = { rpc, rpcSubscriptions };

  // Get or create keypairs
  const { testWallet, usdcMint, destinationAddress } =
    await createKeyPairSigners();
  const mintAuthority = testWallet; // test wallet can be used as mint authority for the test

  // Airdrop SOL to test sender and kora wallets
  await Promise.all([
    airdrop({
      commitment,
      lamports: lamports(solDropAmount),
      recipientAddress: koraAddress,
    }),
    airdrop({
      commitment,
      lamports: lamports(solDropAmount),
      recipientAddress: testWallet.address,
    }),
  ]);

  // Initialize token and ATAs
  await initializeToken({
    client,
    mintAuthority,
    payer: mintAuthority,
    owner: testWallet,
    mint: usdcMint,
    dropAmount: tokenDropAmount,
    decimals: tokenDecimals,
    otherAtaWallets: [testWallet.address, koraAddress, destinationAddress],
  });

  return {
    koraClient: new KoraClient(koraRpcUrl),
    testWallet,
    usdcMint: usdcMint.address,
    destinationAddress,
    koraAddress,
  };
}

export default setupTestSuit;
