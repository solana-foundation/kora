import {
    Address,
    assertIsAddress,
    createKeyPairSignerFromBytes,
    getBase58Encoder,
    KeyPairSigner,
    lamports,
} from '@solana/kit';
import { createClient } from '@solana/kit-client-litesvm';
import { tokenProgram, associatedTokenProgram } from '@solana-program/token';
import { config } from 'dotenv';
import path from 'path';

import { KoraClient } from '../src/index.js';

config({ path: path.resolve(process.cwd(), '.env') });

const DEFAULTS = {
    DECIMALS: 6,

    // Make sure this matches the USDC mint in kora.toml (9BgeTKqmFsPVnfYscfM6NvsgmZxei7XfdciShQ6D3bxJ)
    DESTINATION_ADDRESS: 'AVmDft8deQEo78bRKcGN5ZMf3hyjeLBK4Rd4xGB46yQM',

    // DO NOT USE THESE KEYPAIRS IN PRODUCTION, TESTING KEYPAIRS ONLY
    KORA_ADDRESS: '7AqpcUvgJ7Kh1VmJZ44rWp2XDow33vswo9VK9VqpPU2d',

    KORA_RPC_URL: 'http://localhost:8080/',

    KORA_SIGNER_TYPE: 'memory',

    // Make sure this matches the kora-rpc signer address on launch (root .env)
    SENDER_SECRET: 'tzgfgSWTE3KUA6qfRoFYLaSfJm59uUeZRDy4ybMrLn1JV2drA1mftiaEcVFvq1Lok6h6EX2C4Y9kSKLvQWyMpS5',

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
    const tokenDecimals = Number(process.env.TOKEN_DECIMALS || DEFAULTS.DECIMALS);
    const tokenDropAmount = Number(process.env.TOKEN_DROP_AMOUNT || DEFAULTS.TOKEN_DROP_AMOUNT);
    const solDropAmount = BigInt(process.env.SOL_DROP_AMOUNT || DEFAULTS.SOL_DROP_AMOUNT);
    const testWalletSecret = process.env.SENDER_SECRET || DEFAULTS.SENDER_SECRET;
    const testUsdcMintSecret = process.env.TEST_USDC_MINT_SECRET || DEFAULTS.TEST_USDC_MINT_SECRET;
    const destinationAddress = process.env.DESTINATION_ADDRESS || DEFAULTS.DESTINATION_ADDRESS;
    assertIsAddress(destinationAddress);
    assertIsAddress(koraAddress);

    return {
        destinationAddress,
        koraAddress,
        koraRpcUrl,
        koraSignerType,
        solDropAmount,
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

async function setupTestSuite(): Promise<TestSuite> {
    const { koraAddress, koraRpcUrl, tokenDecimals, tokenDropAmount, solDropAmount } = loadEnvironmentVariables();

    const authConfig =
        process.env.ENABLE_AUTH === 'true'
            ? {
                  apiKey: process.env.KORA_API_KEY || 'test-api-key-123',
                  hmacSecret: process.env.KORA_HMAC_SECRET || 'test-hmac-secret-456',
              }
            : undefined;

    const { testWallet, usdcMint, destinationAddress } = await createKeyPairSigners();
    const client = await createClient({ payer: testWallet }).use(tokenProgram()).use(associatedTokenProgram());

    // Airdrop SOL via LiteSVM
    await client.airdrop(koraAddress, lamports(solDropAmount));
    await client.airdrop(testWallet.address, lamports(solDropAmount));
    await client.airdrop(destinationAddress, lamports(solDropAmount));

    // Create mint
    await client.token.instructions
        .createMint({
            newMint: usdcMint,
            decimals: tokenDecimals,
            mintAuthority: testWallet.address,
        })
        .sendTransaction();

    // Mint tokens to testWallet's ATA (auto-creates ATA)
    await client.token.instructions
        .mintToATA({
            mint: usdcMint.address,
            owner: testWallet.address,
            mintAuthority: testWallet,
            amount: BigInt(tokenDropAmount * 10 ** tokenDecimals),
            decimals: tokenDecimals,
        })
        .sendTransaction();

    // Create ATAs for kora and destination wallets
    for (const owner of [koraAddress, destinationAddress]) {
        await client.associatedToken.instructions
            .createAssociatedTokenIdempotent({
                owner,
                mint: usdcMint.address,
            })
            .sendTransaction();
    }

    return {
        destinationAddress,
        koraAddress,
        koraClient: new KoraClient({ rpcUrl: koraRpcUrl, ...authConfig }),
        koraRpcUrl,
        testWallet,
        usdcMint: usdcMint.address,
    };
}

export default setupTestSuite;
