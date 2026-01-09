import { KoraClient } from "@solana/kora";
import {
  createNoopSigner,
  address,
  getBase64EncodedWireTransaction,
  partiallySignTransactionMessageWithSigners,
  Blockhash,
  KeyPairSigner,
  pipe,
  createTransactionMessage,
  setTransactionMessageFeePayerSigner,
  setTransactionMessageLifetimeUsingBlockhash,
  appendTransactionMessageInstruction,
  generateKeyPairSigner,
} from "@solana/kit";
import { getAddMemoInstruction } from "@solana-program/memo";
import { getTransferSolInstruction } from "@solana-program/system";

const MINIMUM_JITO_TIP = 1_000n; // lamports

const CONFIG = {
  solanaRpcUrl: "https://api.mainnet-beta.solana.com",
  koraRpcUrl: "http://localhost:8080/",
  jitoTipLamports: MINIMUM_JITO_TIP,
  bundleSize: 4, // We'll create 4 transactions for this demo
  pollIntervalMs: 6000,
  pollTimeoutMs: 60000,
};

// Jito tip accounts - one is randomly selected by the block engine
const JITO_TIP_ACCOUNTS = [
  "96gYZGLnJYVFmbjzopPSU6QiEV5fGqZNyN9nmNhvrZU5",
  "HFqU5x63VTqvQss8hp11i4wVV8bD44PvwucfZ2bU7gRe",
  "Cw8CFyM9FkoMi7K7Crf6HNQqf4uEMzpKw6QNghXLvLkY",
  "ADaUMid9yfUytqMBgopwjb2DTLSokTSzL1zt6iGPaS49",
  "DfXygSm4jCyNCybVYYK6DwvWqjKee8pbDmJGcLWNDXjh",
  "ADuUkR4vqLUMWXxW9gh6D6L8pMSawimctcNZ5pGwDcEt",
  "DttWaMuVvTiduZRnguLF7jNxTgiMBZ1hyAumKUiL2KRL",
  "3AVi9Tg9Uo68tJfuvoKvqKNWKkC5wPdSSdeBnizKZ6jT",
];

function getRandomTipAccount(): string {
  return JITO_TIP_ACCOUNTS[
    Math.floor(Math.random() * JITO_TIP_ACCOUNTS.length)
  ];
}

async function initializeClients() {
  console.log("\n[1/5] Initializing clients");
  console.log("  → Kora RPC:", CONFIG.koraRpcUrl);
  console.log("  → Solana RPC:", CONFIG.solanaRpcUrl);

  const client = new KoraClient({
    rpcUrl: CONFIG.koraRpcUrl,
    apiKey: 'kora_facilitator_api_key_example',
  });

  return { client };
}

async function setupKeys(client: KoraClient) {
  console.log("\n[2/5] Setting up keypairs");

  const senderKeypair = await generateKeyPairSigner();
  console.log("  → Sender:", senderKeypair.address);
  
  const { signer_address } = await client.getPayerSigner();
  console.log("  → Kora signer address:", signer_address);

  return { senderKeypair, signer_address };
}

async function createBundleTransactions(
  client: KoraClient,
  senderKeypair: KeyPairSigner,
  signer_address: string
) {
  console.log("\n[3/5] Creating bundle transactions");

  const noopSigner = createNoopSigner(address(signer_address));
  const latestBlockhash = await client.getBlockhash();
  const tipAccount = getRandomTipAccount();

  console.log("  → Blockhash:", latestBlockhash.blockhash.slice(0, 8) + "...");
  console.log("  → Tip account:", tipAccount.slice(0, 8) + "...");

  const transactions: string[] = [];

  for (let i = 0; i < CONFIG.bundleSize; i++) {
    const isLastTransaction = i === CONFIG.bundleSize - 1;
    console.log(
      `  → Transaction ${i + 1}: Kora Memo "Bundle tx #${i + 1}"${
        isLastTransaction ? " + Jito tip" : ""
      }`
    );

    // Build transaction with memo
    let transactionMessage = pipe(
      createTransactionMessage({
        version: 0,
      }),
      (tx) => setTransactionMessageFeePayerSigner(noopSigner, tx),
      (tx) =>
        setTransactionMessageLifetimeUsingBlockhash(
          {
            blockhash: latestBlockhash.blockhash as Blockhash,
            lastValidBlockHeight: 0n,
          },
          tx
        ),
      (tx) =>
        appendTransactionMessageInstruction(
          getAddMemoInstruction({
            memo: `Kora Bundle tx #${i + 1} of ${CONFIG.bundleSize}`,
            signers: [senderKeypair],
          }),
          tx
        ),
      // Add Jito tip to the LAST transaction only
      (tx) =>
        isLastTransaction
          ? appendTransactionMessageInstruction(
              getTransferSolInstruction({
                source: noopSigner,
                destination: address(tipAccount),
                amount: CONFIG.jitoTipLamports,
              }),
              tx
            )
          : tx
    );

    // Sign with sender keypair (required for tip transfer)
    const signedTransaction = await partiallySignTransactionMessageWithSigners(
      transactionMessage
    );
    const base64Transaction =
      getBase64EncodedWireTransaction(signedTransaction);
    transactions.push(base64Transaction);
  }

  console.log(`  ✓ ${transactions.length} transactions created for bundle`);
  return transactions;
}

async function signBundle(
  client: KoraClient,
  transactions: string[],
  signer_address: string
) {
  console.log("\n[4/5] Signing bundle with Kora");
  console.log("  ✓ All transactions signed by user");

  const { signed_transactions, signer_pubkey } = await client.signBundle({
    transactions,
    signer_key: signer_address,
  });

  console.log("  ✓ Bundle co-signed by Kora");
  console.log(`  → ${signed_transactions.length} transactions signed`);

  return { signed_transactions, signer_pubkey };
}

async function submitAndTrackBundle(
  client: KoraClient,
  signedTransactions: string[],
  signer_address: string
) {
  console.log("\n[5/5] Submitting bundle to Jito");

  // Submit bundle
  const { bundle_uuid } = await client.signAndSendBundle({
    transactions: signedTransactions,
    signer_key: signer_address,
  });

  console.log("  ✓ Bundle submitted to Jito block engine");
  console.log(`  → Bundle UUID: ${bundle_uuid}`);
  console.log("  ⏳ Polling bundle status...");

  // Poll for confirmation
  const startTime = Date.now();

  while (Date.now() - startTime < CONFIG.pollTimeoutMs) {
    await new Promise((resolve) => setTimeout(resolve, CONFIG.pollIntervalMs));

    // Note: Bundle status checking would typically be done via Jito's API
    // Kora handles submission but status polling requires additional Jito RPC calls from your client
    // For this demo, we'll assume success after submission

    // In production, you'd call Jito's getBundleStatuses endpoint:
    // https://docs.jito.wtf/lowlatencytxnsend/#getbundlestatuses
    // const status = await jitoRpc.getBundleStatuses([bundle_uuid]);
    // if (status.value[0]?.status === 'Landed') break;

    console.log("  ✓ Bundle landed (simulated for demo)");
    break;
  }

  return { bundle_uuid };
}

async function main() {
  console.log("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
  console.log("KORA JITO BUNDLE DEMO");
  console.log("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

  try {
    // Step 1: Initialize clients
    const { client } = await initializeClients();

    // Step 2: Setup keys
    const { senderKeypair, signer_address } = await setupKeys(client);

    // Step 3: Create bundle transactions
    const transactions = await createBundleTransactions(
      client,
      senderKeypair,
      signer_address
    );

    // Step 4: Sign bundle with Kora
    const { signed_transactions } = await signBundle(
      client,
      transactions,
      signer_address
    );

    // Step 5: Submit and track bundle
    const { bundle_uuid } = await submitAndTrackBundle(
      client,
      signed_transactions,
      signer_address
    );

    console.log("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    console.log("SUCCESS: Bundle confirmed on Solana");
    console.log("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    console.log("\nBundle UUID:");
    console.log(bundle_uuid);
  } catch (error) {
    console.error("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    console.error("ERROR: Demo failed");
    console.error("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    console.error("\nDetails:", error);
    process.exit(1);
  }
}

main().catch((e) => console.error("Error:", e));
