
import { KoraClient } from "@kora/sdk";
import { getAddMemoInstruction } from "@solana-program/memo";
import { getTransferSolInstruction } from "@solana-program/system";
import { deserializedBase64Message, getInstructionsFromBase64Message } from "./helpers";
import { generateKeyPairSigner, createKeyPairSignerFromBytes, getBase58Encoder } from "@solana/kit";
import dotenv from "dotenv";
import path from "path";

dotenv.config({path: path.join(process.cwd(), '..', '.env')});

async function getEnvKeyPair(envKey: string) {
    if (!process.env[envKey]) {
        throw new Error(`Environment variable ${envKey} is not set`);
    }
    const base58Encoder = getBase58Encoder();
    const b58SecretEncoded = base58Encoder.encode(process.env[envKey]);
    return await createKeyPairSignerFromBytes(b58SecretEncoded);
}
async function main() {
    const testSenderKeypair = await getEnvKeyPair('TEST_SENDER_KEYPAIR');
    const destinationKeypair = await getEnvKeyPair('DESTINATION_KEYPAIR');
    const randomKeypair = await generateKeyPairSigner();

    const client = new KoraClient({
        rpcUrl: 'http://localhost:8080/',
        // apiKey: process.env.KORA_API_KEY, // Uncomment if you have authentication enabled in your kora.toml
        // hmacSecret: process.env.KORA_HMAC_SECRET, // Uncomment if you have authentication enabled in your kora.toml
    });

    try {
        const config = await client.getConfig();
        console.log('Kora Config:', config);
        const blockhash = await client.getBlockhash();
        console.log('Blockhash: ', blockhash.blockhash);
        const supportedTokens = await client.getSupportedTokens();
        console.log('Supported Tokens: ', supportedTokens);
        const transferTokens = await client.transferTransaction({
            amount: 10_000_000, // 0.01 SOL (9 decimals)
            token: '9BgeTKqmFsPVnfYscfM6NvsgmZxei7XfdciShQ6D3bxJ', // SOL mint address
            source: testSenderKeypair.address,
            destination: randomKeypair.address // use random to force kora to create ATA init instruction
        });
        const transferSol = await client.transferTransaction({
            amount: 10_000_000, // 0.01 SOL (9 decimals)
            token: '11111111111111111111111111111111', // SOL mint address
            source: testSenderKeypair.address,
            destination: destinationKeypair.address
        });
        const memoInstruction = getAddMemoInstruction({
            memo: 'Hello, Kora!',
        });
        const transferInstructions = getInstructionsFromBase64Message(transferTokens.message);
        const transferSolInstructions = getInstructionsFromBase64Message(transferSol.message);
        const instructions = [...transferInstructions, ...transferSolInstructions, memoInstruction];
        console.log('All Instructions: ', instructions);
        const { payment_address, signer_address } = await client.getPayerSigner();
        console.log('Payment Address: ', payment_address);
        console.log('Signer Address: ', signer_address);
        return;
    } catch (error) {
        console.error(error);
    }
}

main().catch(e => console.error('Error:', e));