import { KoraClient } from "../src";
import {
  getBase64Encoder,
  getBase58Encoder,
  getTransactionDecoder,
  createKeyPairSignerFromBytes,
  KeyPairSigner,
  Transaction,
  signTransaction,
  getBase64EncodedWireTransaction,
} from "@solana/kit";

function transactionFromBase64(base64: string): Transaction {
  const encoder = getBase64Encoder();
  const decoder = getTransactionDecoder();
  const messageBytes = encoder.encode(base64);
  return decoder.decode(messageBytes);
}

async function loadKeypairSignerFromEnvironmentBase58(
  envVar: string
): Promise<KeyPairSigner> {
  const privateKey = process.env[envVar];
  if (!privateKey) {
    throw new Error(`Environment variable ${envVar} is not set`);
  }
  const privateKeyBytes = getBase58Encoder().encode(privateKey);
  return createKeyPairSignerFromBytes(privateKeyBytes);
}

async function main() {
  // Initialize the client with your RPC endpoint
  const rpcUrl = process.env.KORA_RPC_URL!;
  const usdcMint = process.env.USDC_MINT!;
  const client = new KoraClient({ rpcUrl });

  try {
    // Get supported tokens
    const { tokens } = await client.getSupportedTokens();
    console.log("Supported tokens:", tokens);

    // Get current configuration
    const config = await client.getConfig();
    console.log("Current configuration:", config);

    // Load signer from env var
    const signer =
      await loadKeypairSignerFromEnvironmentBase58("PRIVATE_KEY");

    // Example transfer
    const { transaction: transactionString } = await client.transferTransaction(
      {
        amount: 1000000, // 1 USDC (6 decimals)
        token: usdcMint, // USDC mint
        source: signer.address.toString(),
        destination: signer.address.toString(), // Sending to self as example
      }
    );

    // Sign the transaction
    const transaction = transactionFromBase64(transactionString);

    // Send signed transaction
    const signedTransaction = await signTransaction(
      [signer.keyPair],
      transaction
    );

    // Send signed transaction
    const signature = await client.signAndSendTransaction({
      transaction: getBase64EncodedWireTransaction(signedTransaction),
    });

    console.log("Transfer signature:", signature);
  } catch (error) {
    console.error("Error:", error);
  }
}

main();