
import { KoraClient } from "@kora/sdk";
import {
    createKeyPairSignerFromBytes,
    getBase58Encoder,
    createNoopSigner,
    address,
    getBase64EncodedWireTransaction,
    partiallySignTransactionMessageWithSigners,
    Blockhash,
    Base64EncodedWireTransaction,
    partiallySignTransaction,
    TransactionVersion,
    Instruction,
    KeyPairSigner,
    Rpc,
    SolanaRpcApi
} from "@solana/kit";
import {
    createTransaction, 
    createSolanaClient,
    getExplorerLink
} from "gill";
import { getAddMemoInstruction } from "@solana-program/memo";
import { createRecentSignatureConfirmationPromiseFactory } from "@solana/transaction-confirmation";
import dotenv from "dotenv";
import path from "path";

dotenv.config({ path: path.join(process.cwd(), '..', '.env') });

const CONFIG = {
    computeUnitLimit: 200_000n,
    computeUnitPrice: 1_000_000n,
    transactionVersion: 0,
    solanaRpcUrl: "http://127.0.0.1:8899",
    koraRpcUrl: "http://localhost:8080/",
}

async function getEnvKeyPair(envKey: string) {
    if (!process.env[envKey]) {
        throw new Error(`Environment variable ${envKey} is not set`);
    }
    const base58Encoder = getBase58Encoder();
    const b58SecretEncoded = base58Encoder.encode(process.env[envKey]);
    return await createKeyPairSignerFromBytes(b58SecretEncoded);
}

async function initializeClients() {
    console.log('\n[1/6] Initializing clients');
    console.log('  → Kora RPC:', CONFIG.koraRpcUrl);
    console.log('  → Solana RPC:', CONFIG.solanaRpcUrl);
    
    const client = new KoraClient({
        rpcUrl: CONFIG.koraRpcUrl,
        // apiKey: process.env.KORA_API_KEY, // Uncomment if you have authentication enabled in your kora.toml
        // hmacSecret: process.env.KORA_HMAC_SECRET, // Uncomment if you have authentication enabled in your kora.toml
    });

    const { rpc, rpcSubscriptions } = createSolanaClient({
        urlOrMoniker: CONFIG.solanaRpcUrl,
    });

    const confirmTransaction = createRecentSignatureConfirmationPromiseFactory({ rpc, rpcSubscriptions });
    
    return { client, rpc, confirmTransaction };
}

async function setupKeys(client: KoraClient) {
    console.log('\n[2/6] Setting up keypairs');
    
    const testSenderKeypair = await getEnvKeyPair('TEST_SENDER_KEYPAIR');
    const destinationKeypair = await getEnvKeyPair('DESTINATION_KEYPAIR');
    const { signer_address } = await client.getPayerSigner();

    console.log('  → Sender:', testSenderKeypair.address);
    console.log('  → Destination:', destinationKeypair.address);
    console.log('  → Kora signer address:', signer_address);
    
    return { testSenderKeypair, destinationKeypair, signer_address };
}

async function createInstructions(
    client: KoraClient, 
    testSenderKeypair: KeyPairSigner, 
    destinationKeypair: KeyPairSigner
) {
    console.log('\n[3/6] Creating demonstration instructions');
    
    const paymentToken = await client.getConfig().then(config => config.validation_config.allowed_spl_paid_tokens[0]);
    console.log('  → Payment token:', paymentToken);

    // Create token transfer (will initialize ATA if needed)
    const transferTokens = await client.transferTransaction({
        amount: 10_000_000, // 10 USDC (6 decimals)
        token: paymentToken,
        source: testSenderKeypair.address,
        destination: destinationKeypair.address // todo replace with a generated address to test ata creation
    });
    console.log('  ✓ Token transfer instruction created');

    // Create SOL transfer
    const transferSol = await client.transferTransaction({
        amount: 10_000_000, // 0.01 SOL (9 decimals)
        token: '11111111111111111111111111111111', // SOL mint address
        source: testSenderKeypair.address,
        destination: destinationKeypair.address
    });
    console.log('  ✓ SOL transfer instruction created');

    // Add memo instruction
    const memoInstruction = getAddMemoInstruction({
        memo: 'Hello, Kora!',
    });
    console.log('  ✓ Memo instruction created');

    const instructions = [
        ...transferTokens.instructions,
        ...transferSol.instructions,
        memoInstruction
    ];
    
    console.log(`  → Total: ${instructions.length} instructions`);
    return { instructions, paymentToken };
}

async function getPaymentInstruction(
    client: KoraClient, 
    instructions: Instruction[],
    testSenderKeypair: KeyPairSigner,
    paymentToken: string
): Promise<{ paymentInstruction: Instruction }> {
    console.log('\n[4/6] Estimating Kora fee and assembling payment instruction');
    
    const { signer_address } = await client.getPayerSigner();
    const noopSigner = createNoopSigner(address(signer_address));
    const latestBlockhash = await client.getBlockhash();
    
    console.log('  → Fee payer:', signer_address.slice(0, 8) + '...');
    console.log('  → Blockhash:', latestBlockhash.blockhash.slice(0, 8) + '...');

    // Create estimate transaction to get payment instruction
    const estimateTransaction = await createTransaction({
        version: CONFIG.transactionVersion as TransactionVersion,
        instructions,
        feePayer: noopSigner,
        latestBlockhash: {
            blockhash: latestBlockhash.blockhash as Blockhash,
            lastValidBlockHeight: 0n,
        },
        computeUnitPrice: CONFIG.computeUnitPrice,
        computeUnitLimit: CONFIG.computeUnitLimit
    });

    const signedEstimateTransaction = await partiallySignTransactionMessageWithSigners(estimateTransaction);
    const base64EncodedWireTransaction = getBase64EncodedWireTransaction(signedEstimateTransaction);
    console.log('  ✓ Estimate transaction built');

    // Get payment instruction from Kora
    const paymentInstruction = await client.getPaymentInstruction({
        transaction: base64EncodedWireTransaction,
        fee_token: paymentToken,
        source_wallet: testSenderKeypair.address,
    });
    console.log('  ✓ Payment instruction received from Kora');
    
    return { paymentInstruction: paymentInstruction.payment_instruction };
}

async function getFinalTransaction(
    client: KoraClient, 
    paymentInstruction: Instruction,
    testSenderKeypair: KeyPairSigner, 
    instructions: Instruction[], 
    signer_address: string
): Promise<Base64EncodedWireTransaction> {
    console.log('\n[5/6] Creating and signing final transaction (with payment)');
    const noopSigner = createNoopSigner(address(signer_address));

    // Build final transaction with payment instruction
    const newBlockhash = await client.getBlockhash();
    const fullTransaction = await createTransaction({
        version: CONFIG.transactionVersion as TransactionVersion,
        instructions: [...instructions, paymentInstruction],
        feePayer: noopSigner,
        latestBlockhash: {
            blockhash: newBlockhash.blockhash as Blockhash,
            lastValidBlockHeight: 0n,
        },
        computeUnitPrice: CONFIG.computeUnitPrice,
        computeUnitLimit: CONFIG.computeUnitLimit
    });
    console.log('  ✓ Final transaction built with payment');

    // Sign with user keypair
    const signedFullTransaction = await partiallySignTransactionMessageWithSigners(fullTransaction);
    const userSignedTransaction = await partiallySignTransaction([testSenderKeypair.keyPair], signedFullTransaction);
    const base64EncodedWireFullTransaction = getBase64EncodedWireTransaction(userSignedTransaction);
    console.log('  ✓ Transaction signed by user');
    
    return base64EncodedWireFullTransaction;
}

async function submitTransaction(
    client: KoraClient, 
    rpc: Rpc<SolanaRpcApi>, 
    confirmTransaction: ReturnType<typeof createRecentSignatureConfirmationPromiseFactory>, 
    signedTransaction: Base64EncodedWireTransaction, 
    signer_address: string
) {
    console.log('\n[6/6] Signing transaction with Kora and sending to Solana cluster');
    
    // Get Kora's signature
    const { signed_transaction } = await client.signTransactionIfPaid({
        transaction: signedTransaction,
        signer_key: signer_address
    });
    console.log('  ✓ Transaction co-signed by Kora');

    // Submit to Solana network
    const signature = await rpc.sendTransaction(signed_transaction as Base64EncodedWireTransaction, {
        encoding: 'base64'
    }).send();
    console.log('  ✓ Transaction submitted to network');
    
    console.log('  ⏳ Awaiting confirmation...');
    await confirmTransaction({
        commitment: 'confirmed',
        signature,
        abortSignal: new AbortController().signal
    });
    
    console.log('\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━');
    console.log('SUCCESS: Transaction confirmed on Solana');
    console.log('━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━');
    console.log('\nTransaction signature:');
    console.log(signature);
    console.log('\nView on explorer:');
    console.log(getExplorerLink({transaction: signature, cluster: 'localhost'}));
    
    return signature;
}

async function main() {
    console.log('\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━');
    console.log('KORA GASLESS TRANSACTION DEMO');
    console.log('━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━');
    
    try {
        // Step 1: Initialize clients
        const { client, rpc, confirmTransaction } = await initializeClients();
        
        // Step 2: Setup keys
        const { testSenderKeypair, destinationKeypair, signer_address } = await setupKeys(client);
        
        // Step 3: Create demo instructions
        const { instructions, paymentToken } = await createInstructions(client, testSenderKeypair, destinationKeypair);
        
        // Step 4: Get payment instruction from Kora
        const { paymentInstruction } = await getPaymentInstruction(client, instructions, testSenderKeypair, paymentToken);
        
        // Step 5: Create and partially sign final transaction
        const finalSignedTransaction = await getFinalTransaction(
            client,  
            paymentInstruction, 
            testSenderKeypair, 
            instructions, 
            signer_address
        );
        
        // Step 6: Get Kora's signature and submit to Solana cluster
        await submitTransaction(client, rpc, confirmTransaction, finalSignedTransaction, signer_address);
    } catch (error) {
        console.error('\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━');
        console.error('ERROR: Demo failed');
        console.error('━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━');
        console.error('\nDetails:', error);
        process.exit(1);
    }
}

main().catch(e => console.error('Error:', e));