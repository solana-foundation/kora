import {
    address,
    Address,
    appendTransactionMessageInstruction,
    appendTransactionMessageInstructions,
    type Blockhash,
    compileTransaction,
    createKeyPairSignerFromBytes,
    createTransactionMessage,
    getBase58Encoder,
    getBase64EncodedWireTransaction,
    type KeyPairSigner,
    partiallySignTransaction,
    pipe,
    setTransactionMessageFeePayerSigner,
    setTransactionMessageLifetimeUsingBlockhash,
    type TransactionSigner,
} from '@solana/kit';
import { findAssociatedTokenPda, getTransferInstruction, TOKEN_PROGRAM_ADDRESS } from '@solana-program/token';

import { KoraClient } from '../src/index.js';

async function loadKeypairSignerFromEnvironmentBase58(envVar: string): Promise<KeyPairSigner> {
    const privateKey = process.env[envVar];
    if (!privateKey) {
        throw new Error(`Environment variable ${envVar} is not set`);
    }
    return createKeyPairSignerFromBytes(getBase58Encoder().encode(privateKey));
}

async function buildTransferInstruction(mint: Address, sender: KeyPairSigner, recipient: Address, amount: bigint) {
    const [source] = await findAssociatedTokenPda({ mint, owner: sender.address, tokenProgram: TOKEN_PROGRAM_ADDRESS });
    const [destination] = await findAssociatedTokenPda({
        mint,
        owner: recipient,
        tokenProgram: TOKEN_PROGRAM_ADDRESS,
    });
    return getTransferInstruction({ amount, authority: sender, destination, source });
}

async function main() {
    const rpcUrl = process.env.KORA_RPC_URL!;
    const usdcMint = address(process.env.USDC_MINT!);
    const client = new KoraClient({ rpcUrl });

    const sender = await loadKeypairSignerFromEnvironmentBase58('PRIVATE_KEY');

    const { signer_address } = await client.getPayerSigner();
    // Kora signs as fee payer server-side, so the client only needs its address here.
    const feePayer = { address: address(signer_address) } as TransactionSigner;

    const transferInstruction = await buildTransferInstruction(usdcMint, sender, sender.address, 1_000_000n);

    const buildMessage = async () => {
        const { blockhash } = await client.getBlockhash();
        return pipe(
            createTransactionMessage({ version: 0 }),
            tx => setTransactionMessageFeePayerSigner(feePayer, tx),
            tx =>
                setTransactionMessageLifetimeUsingBlockhash(
                    { blockhash: blockhash as Blockhash, lastValidBlockHeight: BigInt(Number.MAX_SAFE_INTEGER) },
                    tx,
                ),
        );
    };

    // The fee is priced against the transaction that will actually run, so estimate
    // against the unpaid message first and rebuild with the payment appended.
    const estimateMessage = appendTransactionMessageInstruction(transferInstruction, await buildMessage());
    const { payment_instruction } = await client.getPaymentInstruction({
        fee_token: usdcMint,
        source_wallet: sender.address,
        transaction: getBase64EncodedWireTransaction(compileTransaction(estimateMessage)),
    });

    const finalMessage = appendTransactionMessageInstructions(
        [transferInstruction, payment_instruction],
        await buildMessage(),
    );
    const signedTransaction = await partiallySignTransaction([sender.keyPair], compileTransaction(finalMessage));

    const { signature } = await client.signAndSendTransaction({
        transaction: getBase64EncodedWireTransaction(signedTransaction),
    });

    console.log('Transfer signature:', signature);
}

main().catch(error => {
    console.error('Error:', error);
    process.exit(1);
});
