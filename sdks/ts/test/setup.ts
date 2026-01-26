import {
    Address,
    airdropFactory,
    appendTransactionMessageInstructions,
    assertIsAddress,
    assertIsSendableTransaction,
    assertIsTransactionWithBlockhashLifetime,
    Commitment,
    createKeyPairSignerFromBytes,
    createSolanaRpc,
    createSolanaRpcSubscriptions,
    createTransactionMessage,
    getBase58Encoder,
    getSignatureFromTransaction,
    Instruction,
    KeyPairSigner,
    lamports,
    MicroLamports,
    pipe,
    Rpc,
    RpcSubscriptions,
    sendAndConfirmTransactionFactory,
    setTransactionMessageFeePayerSigner,
    setTransactionMessageLifetimeUsingBlockhash,
    Signature,
    signTransactionMessageWithSigners,
    SolanaRpcApi,
    SolanaRpcSubscriptionsApi,
    TransactionMessage,
    TransactionMessageWithBlockhashLifetime,
    TransactionMessageWithFeePayer,
    TransactionSigner,
} from '@solana/kit';
import {
    MAX_COMPUTE_UNIT_LIMIT,
    updateOrAppendSetComputeUnitLimitInstruction,
    updateOrAppendSetComputeUnitPriceInstruction,
} from '@solana-program/compute-budget';
import { getCreateAccountInstruction } from '@solana-program/system';
import {
    findAssociatedTokenPda,
    getCreateAssociatedTokenIdempotentInstructionAsync,
    getInitializeMintInstruction,
    getMintSize,
    getMintToInstruction,
    TOKEN_PROGRAM_ADDRESS,
} from '@solana-program/token';
import { config } from 'dotenv';
import path from 'path';

import { KoraClient } from '../src/index.js';

config({ path: path.resolve(process.cwd(), '.env') });

const DEFAULTS = {
    COMMITMENT: 'processed' as Commitment,
    DECIMALS: 6,

    // Make sure this matches the USDC mint in kora.toml (9BgeTKqmFsPVnfYscfM6NvsgmZxei7XfdciShQ6D3bxJ)
    DESTINATION_ADDRESS: 'AVmDft8deQEo78bRKcGN5ZMf3hyjeLBK4Rd4xGB46yQM',

    // DO NOT USE THESE KEYPAIRS IN PRODUCTION, TESTING KEYPAIRS ONLY
    KORA_ADDRESS: '7AqpcUvgJ7Kh1VmJZ44rWp2XDow33vswo9VK9VqpPU2d',

    KORA_RPC_URL: 'http://localhost:8080/',

    KORA_SIGNER_TYPE: 'memory',

    // Make sure this matches the kora-rpc signer address on launch (root .env)
    SENDER_SECRET: 'tzgfgSWTE3KUA6qfRoFYLaSfJm59uUeZRDy4ybMrLn1JV2drA1mftiaEcVFvq1Lok6h6EX2C4Y9kSKLvQWyMpS5',

    SOLANA_RPC_URL: 'http://127.0.0.1:8899',

    SOLANA_WS_URL: 'ws://127.0.0.1:8900',

    SOL_DROP_AMOUNT: 1_000_000_000,

    // HhA5j2rRiPbMrpF2ZD36r69FyZf3zWmEHRNSZbbNdVjf
    TEST_USDC_MINT_SECRET: '59kKmXphL5UJANqpFFjtH17emEq3oRNmYsx6a3P3vSGJRmhMgVdzH77bkNEi9bArRViT45e8L2TsuPxKNFoc3Qfg',

    TOKEN_DROP_AMOUNT: 100_000, // Default signer type
};

interface TestSuite {
    destinationAddress: Address<string>;
    koraAddress: Address<string>;
    koraClient: KoraClient;
    koraRpcUrl: string;
    testWallet: KeyPairSigner<string>;
    usdcMint: Address<string>;
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
// TODO Add KORA_PRIVATE_KEY_2= support for multi-signer configs
export function loadEnvironmentVariables() {
    const koraSignerType = process.env.KORA_SIGNER_TYPE || DEFAULTS.KORA_SIGNER_TYPE;

    let koraAddress = process.env.KORA_ADDRESS;
    if (!koraAddress) {
        switch (koraSignerType) {
            case 'turnkey':
                koraAddress = process.env.TURNKEY_PUBLIC_KEY;
                if (!koraAddress) {
                    throw new Error('TURNKEY_PUBLIC_KEY must be set when using Turnkey signer');
                }
                break;
            case 'privy':
                koraAddress = process.env.PRIVY_PUBLIC_KEY;
                if (!koraAddress) {
                    throw new Error('PRIVY_PUBLIC_KEY must be set when using Privy signer');
                }
                break;
            case 'memory':
            default:
                koraAddress = DEFAULTS.KORA_ADDRESS;
                break;
        }
    }

    const koraRpcUrl = process.env.KORA_RPC_URL || DEFAULTS.KORA_RPC_URL;
    const solanaRpcUrl = process.env.SOLANA_RPC_URL || DEFAULTS.SOLANA_RPC_URL;
    const solanaWsUrl = process.env.SOLANA_WS_URL || DEFAULTS.SOLANA_WS_URL;
    const commitment = (process.env.COMMITMENT || DEFAULTS.COMMITMENT) as Commitment;
    const tokenDecimals = Number(process.env.TOKEN_DECIMALS || DEFAULTS.DECIMALS);
    const tokenDropAmount = Number(process.env.TOKEN_DROP_AMOUNT || DEFAULTS.TOKEN_DROP_AMOUNT);
    const solDropAmount = BigInt(process.env.SOL_DROP_AMOUNT || DEFAULTS.SOL_DROP_AMOUNT);
    const testWalletSecret = process.env.SENDER_SECRET || DEFAULTS.SENDER_SECRET;
    const testUsdcMintSecret = process.env.TEST_USDC_MINT_SECRET || DEFAULTS.TEST_USDC_MINT_SECRET;
    const destinationAddress = process.env.DESTINATION_ADDRESS || DEFAULTS.DESTINATION_ADDRESS;
    assertIsAddress(destinationAddress);
    assertIsAddress(koraAddress);

    return {
        commitment,
        destinationAddress,
        koraAddress,
        koraRpcUrl,
        koraSignerType,
        solDropAmount,
        solanaRpcUrl,
        solanaWsUrl,
        testUsdcMintSecret,
        testWalletSecret,
        tokenDecimals,
        tokenDropAmount,
    };
}

async function createKeyPairSigners() {
    const { testWalletSecret, testUsdcMintSecret, destinationAddress } = loadEnvironmentVariables();
    const testWallet = await createKeyPairSignerFromB58Secret(testWalletSecret);
    const usdcMint = await createKeyPairSignerFromB58Secret(testUsdcMintSecret);
    return {
        destinationAddress,
        testWallet,
        usdcMint,
    };
}

const createDefaultTransaction = async (
    client: Client,
    feePayer: TransactionSigner,
    computeLimit: number = MAX_COMPUTE_UNIT_LIMIT,
    feeMicroLamports: MicroLamports = 1n as MicroLamports,
): Promise<TransactionMessage & TransactionMessageWithBlockhashLifetime & TransactionMessageWithFeePayer> => {
    const { value: latestBlockhash } = await client.rpc.getLatestBlockhash().send();
    return pipe(
        createTransactionMessage({ version: 0 }),
        tx => setTransactionMessageFeePayerSigner(feePayer, tx),
        tx => setTransactionMessageLifetimeUsingBlockhash(latestBlockhash, tx),
        tx => updateOrAppendSetComputeUnitPriceInstruction(feeMicroLamports, tx),
        tx => updateOrAppendSetComputeUnitLimitInstruction(computeLimit, tx),
    );
};

const signAndSendTransaction = async (
    client: Client,
    transactionMessage: TransactionMessage & TransactionMessageWithBlockhashLifetime & TransactionMessageWithFeePayer,
    commitment: Commitment,
) => {
    const signedTransaction = await signTransactionMessageWithSigners(transactionMessage);
    const signature = getSignatureFromTransaction(signedTransaction);
    assertIsSendableTransaction(signedTransaction);
    assertIsTransactionWithBlockhashLifetime(signedTransaction);
    await sendAndConfirmTransactionFactory(client)(signedTransaction, { commitment, skipPreflight: true });
    return signature;
};

function safeStringify(obj: any) {
    return JSON.stringify(
        obj,
        (key, value) => {
            if (typeof value === 'bigint') {
                return value.toString();
            }
            return value;
        },
        2,
    );
}

async function sendAndConfirmInstructions(
    client: Client,
    payer: TransactionSigner,
    instructions: Instruction[],
    description: string,
    commitment: Commitment = loadEnvironmentVariables().commitment,
): Promise<Signature> {
    try {
        const signature = await pipe(
            await createDefaultTransaction(client, payer, 200_000),
            tx => appendTransactionMessageInstructions(instructions, tx),
            tx => signAndSendTransaction(client, tx, commitment),
        );
        return signature;
    } catch (error) {
        console.error(safeStringify(error));
        throw new Error(
            `Failed to ${description.toLowerCase()}: ${error instanceof Error ? error.message : 'Unknown error'}`,
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
    decimals: number;
    dropAmount: number;
    mint: KeyPairSigner<string>;
    mintAuthority: KeyPairSigner<string>;
    otherAtaWallets?: Address<string>[];
    owner: KeyPairSigner<string>;
    payer: KeyPairSigner<string>;
}) {
    // Get Owner ATA
    const [ata] = await findAssociatedTokenPda({
        mint: mint.address,
        owner: owner.address,
        tokenProgram: TOKEN_PROGRAM_ADDRESS,
    });
    // Get Mint size & rent
    const mintSpace = BigInt(getMintSize());
    const mintRent = await client.rpc.getMinimumBalanceForRentExemption(mintSpace).send();
    // Create instructions for new token mint
    const baseInstructions = [
        // Create the Mint Account
        getCreateAccountInstruction({
            lamports: mintRent,
            newAccount: mint,
            payer,
            programAddress: TOKEN_PROGRAM_ADDRESS,
            space: mintSpace,
        }),
        // Initialize the Mint
        getInitializeMintInstruction({
            decimals,
            mint: mint.address,
            mintAuthority: mintAuthority.address,
        }),
        // Create Associated Token Account
        await getCreateAssociatedTokenIdempotentInstructionAsync({
            mint: mint.address,
            owner: owner.address,
            payer,
        }),
        // Mint To the Destination Associated Token Account
        getMintToInstruction({
            amount: BigInt(dropAmount * 10 ** decimals),
            mint: mint.address,
            mintAuthority,
            token: ata,
        }),
    ];
    // Generate Create ATA instructions for other token accounts we wish to add
    const otherAtaInstructions = otherAtaWallets
        ? await Promise.all(
              otherAtaWallets.map(
                  async wallet =>
                      await getCreateAssociatedTokenIdempotentInstructionAsync({
                          mint: mint.address,
                          owner: wallet,
                          payer,
                      }),
              ),
          )
        : [];
    const alreadyExists = await mintExists(client, mint.address);
    const instructions = alreadyExists ? [...otherAtaInstructions] : [...baseInstructions, ...otherAtaInstructions];
    await sendAndConfirmInstructions(client, payer, instructions, 'Initialize token and ATAs', 'finalized');
}

async function setupTestSuite(): Promise<TestSuite> {
    const { koraAddress, koraRpcUrl, tokenDecimals, tokenDropAmount, solDropAmount, solanaRpcUrl, solanaWsUrl } =
        loadEnvironmentVariables();

    // Load auth config from environment if not provided
    const authConfig =
        process.env.ENABLE_AUTH === 'true'
            ? {
                  apiKey: process.env.KORA_API_KEY || 'test-api-key-123',
                  hmacSecret: process.env.KORA_HMAC_SECRET || 'test-hmac-secret-456',
              }
            : undefined;

    // Create Solana client
    const rpc = createSolanaRpc(solanaRpcUrl);
    const rpcSubscriptions = createSolanaRpcSubscriptions(solanaWsUrl);
    const airdrop = airdropFactory({ rpc, rpcSubscriptions });
    const client: Client = { rpc, rpcSubscriptions };

    // Get or create keypairs
    const { testWallet, usdcMint, destinationAddress } = await createKeyPairSigners();
    const mintAuthority = testWallet; // test wallet can be used as mint authority for the test

    // Airdrop SOL to test sender and kora wallets
    await Promise.all([
        airdrop({
            commitment: 'finalized',
            lamports: lamports(solDropAmount),
            recipientAddress: koraAddress,
        }),
        airdrop({
            commitment: 'finalized',
            lamports: lamports(solDropAmount),
            recipientAddress: testWallet.address,
        }),
    ]);

    // Initialize token and ATAs
    await initializeToken({
        client,
        decimals: tokenDecimals,
        dropAmount: tokenDropAmount,
        mint: usdcMint,
        mintAuthority,
        otherAtaWallets: [testWallet.address, koraAddress, destinationAddress],
        owner: testWallet,
        payer: mintAuthority,
    });

    return {
        destinationAddress,
        koraAddress,
        koraClient: new KoraClient({ rpcUrl: koraRpcUrl, ...authConfig }),
        koraRpcUrl,
        testWallet,
        usdcMint: usdcMint.address,
    };
}

const mintExists = async (client: Client, mint: Address<string>) => {
    try {
        const mintAccount = await client.rpc.getAccountInfo(mint).send();
        return mintAccount.value !== null;
    } catch {
        return false;
    }
};

export default setupTestSuite;
