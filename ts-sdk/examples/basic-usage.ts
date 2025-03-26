import { KoraClient } from '../src';
// Note: you will have to install web3.js & bs58 to use this
// the sdk does not require any dependencies so none have been added to package.json
import { Keypair, VersionedTransaction } from '@solana/web3.js';
import {default as bs58} from 'bs58';

async function main() {
  // Initialize the client with your RPC endpoint
  const rpcUrl = process.env.KORA_RPC_URL!;
  const usdcMint = process.env.USDC_MINT!;
  const client = new KoraClient(rpcUrl);

  try {
    // Get supported tokens
    const { tokens } = await client.getSupportedTokens();
    console.log('Supported tokens:', tokens);

    // Get current configuration
    const config = await client.getConfig();
    console.log('Current configuration:', config);

    // Load keypair from env var
    const privateKeyBytes = bs58.decode(process.env.SDK_PRIVATE_KEY!);
    const keypair = Keypair.fromSecretKey(privateKeyBytes);

    // Example transfer
    const { transaction } = await client.transferTransaction({
      amount: 1000000, // 1 USDC (6 decimals)
      token: usdcMint, // USDC mint
      source: keypair.publicKey.toString(),
      destination: keypair.publicKey.toString() // Sending to self as example
    });

    // Sign the transaction
    const decodedTransaction = VersionedTransaction.deserialize(bs58.decode(transaction));
    const signedTransaction = decodedTransaction;
    signedTransaction.sign([keypair]);

    // Send signed transaction
    const signature = await client.signAndSendTransaction({ transaction: bs58.encode(signedTransaction.serialize()) });

    console.log('Transfer signature:', signature);

  } catch (error) {
    console.error('Error:', error);
  }
}

main();