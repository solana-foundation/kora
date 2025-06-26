import {
  deriveAttestationPda,
  deriveCredentialPda,
    deriveSchemaPda,
    fetchSchema,
    getCreateAttestationInstruction,
    serializeAttestationData,
    type CreateAttestationInput,
  } from "sas-lib";
  import {
    sendAndConfirmTransactionFactory,
    createKeyPairFromBytes,
    type Address,
    createSolanaRpc,
    type Rpc,
    type SolanaRpcApi,
    createSolanaRpcSubscriptions,
    type RpcSubscriptions,
    type SolanaRpcSubscriptionsApi,
    appendTransactionMessageInstruction,
    setTransactionMessageLifetimeUsingBlockhash,
    setTransactionMessageFeePayer,
    createTransactionMessage,
    pipe,
    getSignatureFromTransaction,
  } from "@solana/kit";
  import { createKeyPairSignerFromPrivateKeyBytes, signTransactionMessageWithSigners } from "@solana/signers";
  import * as bs58 from "bs58";

  async function createAttestation() {

    type RpcClient = {
      rpc: Rpc<SolanaRpcApi>;
      rpcSubscriptions: RpcSubscriptions<SolanaRpcSubscriptionsApi>;
    };
  
  
    const createDefaultSolanaClient = (): RpcClient => {
      const rpc = createSolanaRpc(`https://${process.env.RPC_ROOT!}`);
      const rpcSubscriptions = createSolanaRpcSubscriptions(`wss://${process.env.RPC_ROOT!}`);
      return { rpc, rpcSubscriptions };
    };
  
    const { rpc, rpcSubscriptions } = createDefaultSolanaClient();
  
    // Generate keypairs using @solana/kit
    const recipient = await createKeyPairSignerFromPrivateKeyBytes(bs58.default.decode(process.env.RECIPIENT_KEYPAIR!));
    const authority = await createKeyPairSignerFromPrivateKeyBytes(bs58.default.decode(process.env.AUTHORITY_KEYPAIR!));
    const operatorKey = await createKeyPairSignerFromPrivateKeyBytes(bs58.default.decode(process.env.OPERATOR_KEYPAIR!));
 
    const credential = await deriveCredentialPda({
        authority: authority.address,
        name: "Koranet Schema"
    })

    const schemaPda = await deriveSchemaPda({
        credential: credential[0] as Address,
        name: "Koranet Schema",
        version: 1
    })

    const schema = await fetchSchema(rpc, schemaPda[0] as Address);

    const attestationPda = await deriveAttestationPda({
        credential: credential[0] as Address,
        schema: schemaPda[0] as Address,
        nonce: operatorKey.address
    })

    const data = {
      "domain": "https://kora-runner.xyz/wif",
      "fee_payer": "G1ajNiQqS962dujnPvzdbQs2aLLuTZ4RUrH4fD1YLn3g",
      "allowed_programs": [
          "11111111111111111111111111111111",
          "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA",
          "ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL"
      ],
      "allowed_tokens": [
          "4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU"
      ],
      "allowed_spl_paid_tokens": [
          "4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU"
      ],
      "max_signatures": 10,
      "max_allowed_lamports": 1000000,
      "disallowed_accounts": [],
      "price_source": "Jupiter",
    }


    const attestationInput: CreateAttestationInput = {
      payer: recipient,
      authority: authority,
      credential: credential[0] as Address,
      schema: schemaPda[0] as Address,
      nonce: operatorKey.address,
      expiry: Math.floor(Date.now() / 1000) + 365 * 24 * 60 * 60, // 1 year from now
      data: serializeAttestationData(schema.data, data),
      attestation: attestationPda[0] as Address
    };
  
    // Create the instruction
    const attestationIx = getCreateAttestationInstruction(attestationInput);
  
    const { value: latestBlockhash } = await rpc.getLatestBlockhash().send();  
  
    const transactionMessage = pipe(
      createTransactionMessage({ version: 0 }),
      tx => setTransactionMessageFeePayer(recipient.address, tx),
      tx => setTransactionMessageLifetimeUsingBlockhash(latestBlockhash, tx),
      tx => appendTransactionMessageInstruction(attestationIx, tx),
    );
  
    const signedTransaction = await signTransactionMessageWithSigners(transactionMessage); 
  
    const sendAndConfirm = sendAndConfirmTransactionFactory({ rpc, rpcSubscriptions });
  
    try {
  
      await sendAndConfirm(signedTransaction, {
        commitment: "confirmed",
        skipPreflight: true
      });
  
      const signature = getSignatureFromTransaction(signedTransaction); 
  
      console.log(`Attestation created with signature: ${signature}`);
      return signature;
    } catch (error) {
      console.error("Error creating attestation:", error);
      throw error;
    }
  }
  
  createAttestation().catch(console.error);