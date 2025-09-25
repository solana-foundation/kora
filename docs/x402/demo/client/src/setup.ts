import { getBase58Decoder, getBase58Encoder, createKeyPairSignerFromBytes, createSolanaRpc, Address, lamports, LAMPORTS_PER_SOL, createSolanaClient } from "gill";
import { appendFile } from 'fs/promises';
import path from "path";


async function createB58SecretKey(): Promise<string> {
    // await assertKeyGenerationIsAvailable();
    const base58Decoder = getBase58Decoder();
    // Create keypair with exportable private key
    // For demo purposes only
    const keyPair = await crypto.subtle.generateKey(
        "Ed25519",  // Algorithm. Native implementation status: https://github.com/WICG/webcrypto-secure-curves/issues/20
        true,       // Allows the private key to be exported (eg for saving it to a file) - public key is always extractable see https://wicg.github.io/webcrypto-secure-curves/#ed25519-operations
        ["sign", "verify"], // Allowed uses
    );

    // Get the raw 32-byte private key
    const pkcs8ArrayBuffer = await crypto.subtle.exportKey("pkcs8", keyPair.privateKey);
    const pkcs8Bytes = new Uint8Array(pkcs8ArrayBuffer);
    const rawPrivateKey = pkcs8Bytes.slice(-32);

    // Get the 32-byte public key
    const publicKeyArrayBuffer = await crypto.subtle.exportKey("raw", keyPair.publicKey);
    const publicKeyBytes = new Uint8Array(publicKeyArrayBuffer);

    // Create Solana-style 64-byte secret key (private + public)
    const solanaSecretKey = new Uint8Array(64);
    solanaSecretKey.set(rawPrivateKey, 0);     // First 32 bytes
    solanaSecretKey.set(publicKeyBytes, 32);   // Next 32 bytes

    const b58Secret = base58Decoder.decode(solanaSecretKey)

    return b58Secret;
}

const createKeyPairSignerFromB58Secret = async (b58Secret: string) => {
    const base58Encoder = getBase58Encoder();
    const b58SecretEncoded = base58Encoder.encode(b58Secret);
    return await createKeyPairSignerFromBytes(b58SecretEncoded);
}

const addKeypairToEnvFile = async (
    variableName: string,
    attemptAirdrop: boolean = false,
    envPath: string = path.join(process.cwd(), '..'),
    envFileName: string = ".env",
    b58Secret?: string,
) => {

    const ADDRESS_SUFFIX = "_ADDRESS";
    const PRIVATE_KEY_SUFFIX = "_PRIVATE_KEY";

    if (!b58Secret) {
        b58Secret = await createB58SecretKey();
    }

    const keypairSigner = await createKeyPairSignerFromB58Secret(b58Secret);
    const addressLog = `\n${variableName}${ADDRESS_SUFFIX}=${keypairSigner.address}\n`;
    const privateKeyLog = `${variableName}${PRIVATE_KEY_SUFFIX}=${b58Secret}\n`;


    const fullPath = path.join(envPath, envFileName);
    try {
        await appendFile(
            fullPath,
            `${addressLog}${privateKeyLog}`,
        );
        console.log(`${variableName}${ADDRESS_SUFFIX} and ${variableName}${PRIVATE_KEY_SUFFIX} added to env file successfully`);
        if (attemptAirdrop) {
            console.warn(`Attempting airdrop on devnet - if you need to run multiple times, you may run into rate limiting. Check out https://faucet.solana.com/ for other options.`)

            await airdrop(keypairSigner.address);
        }
        return keypairSigner;
    } catch (e) {
        throw e;
    }
};

async function airdrop(address: Address) {
    const client = createSolanaClient({ urlOrMoniker: 'devnet' });
    await client.rpc.requestAirdrop(
        address,
        lamports(BigInt(LAMPORTS_PER_SOL / 10)),
        { commitment: 'processed' }
    ).send();
}

async function test() {
    await addKeypairToEnvFile('KORA_SIGNER');
    await addKeypairToEnvFile('PAYER');
}
test();