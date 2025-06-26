import {
    CreateCredentialInput,
    CreateSchemaInput,
    deriveCredentialPda,
    deriveSchemaPda,
    fetchSchema,
    getCreateAttestationInstruction,
    getCreateCredentialInstruction,
    getCreateSchemaInstruction,
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
  import { schema_struct_serialize } from "./helper.js";

  async function createSchema() {

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
  
    const payer = await createKeyPairSignerFromPrivateKeyBytes(bs58.default.decode(process.env.PAYER_KEYPAIR!));
    const authority = await createKeyPairSignerFromPrivateKeyBytes(bs58.default.decode(process.env.AUTHORITY_KEYPAIR!));

    const [credentialPda, credentialBump] = await deriveCredentialPda({authority: authority.address, name: "Koranet Credential"});
    const [schemaPda, schemaBump] = await deriveSchemaPda({credential: credentialPda, name: "Koranet Schema", version: 1});


    // stub kora config
    const schema = {
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

    const { fieldNames, valueTypes, byteLayout } = schema_struct_serialize(schema);

    const schemaInput: CreateSchemaInput = {
        payer: payer,
        authority: authority,
        credential: credentialPda,
        schema: schemaPda,
        name: "Koranet Schema",
        description: "Schema for koranet",
        layout: byteLayout,
        fieldNames: fieldNames,
    };
  
    const schemaIx = getCreateSchemaInstruction(schemaInput);
  
    const { value: latestBlockhash } = await rpc.getLatestBlockhash().send();  
  
    const transactionMessage = pipe(
      createTransactionMessage({ version: 0 }),
      tx => setTransactionMessageFeePayer(payer.address, tx),
      tx => setTransactionMessageLifetimeUsingBlockhash(latestBlockhash, tx),
      tx => appendTransactionMessageInstruction(schemaIx, tx),
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
  
  createSchema().catch(console.error);