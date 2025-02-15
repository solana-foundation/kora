/* eslint-disable @typescript-eslint/no-unused-vars */
import {
    ActionPostResponse,
    ActionGetResponse,
    ActionPostRequest,
    ACTIONS_CORS_HEADERS,
  } from "@solana/actions";
  import {
    clusterApiUrl,
    Connection,
    PublicKey,
  } from "@solana/web3.js";
import { bs58 } from "bs58";

  export const GET = async (req: Request) => {

    const payload: ActionGetResponse = {
      title: "Kora TransferTransaction RPC Method Blink",
      icon: "https://ucarecdn.com/7aa46c85-08a4-4bc7-9376-88ec48bb1f43/-/preview/880x864/-/quality/smart/-/format/auto/",
      description: ``,
      label: "TransferTxn",
      links: {
        actions: [
            {
              label: `Send Tokens`,
              href: `/api/action/transferTx?to={recipientAddress}&amount={amount}&tokenMint={tokenMint}`,
              type: "post",
              parameters: [{
                name: "recipientAddress",
                label: "Address of the receiver",
                required: true
            },
            {
              name: "tokenMint",
              label: "Token Mint Address",
              required: true
            },
            {
              name: "amount",
              label: "Amount of tokens",
              required: true
            },
          ],
            },
          ],
      },
    };

    return Response.json(payload, {
        headers: ACTIONS_CORS_HEADERS,
      });
  }

  export const POST = async(req: Request) => {
    try{

      const requestUrl = new URL(req.url);
      const { recipientAddress, tokenMint, amount } = validatedQueryParams(requestUrl); //decoding query params
  
      const body: ActionPostRequest = await req.json(); //the POST request body
  
      // validate the client provided input
      let account: PublicKey;
      try {
        account = new PublicKey(body.account);
      } catch (err) {
        return new Response('Invalid "account" provided', {
          status: 400,
          headers: ACTIONS_CORS_HEADERS,
        });
      }

    const connection = new Connection(clusterApiUrl("devnet"), "confirmed");
    const tokenInfo = await connection.getTokenSupply(new PublicKey(tokenMint));
    const tokenDecimals = tokenInfo.value.decimals || 9;
    const amountInLamports = amount * (10**tokenDecimals);

    //make sure the kora-rpc server is running on port 8080
    const rpcResponse = await fetch('http://127.0.0.1:8080', {
      method: 'POST',
      headers: {
          'Content-Type': 'application/json',
      },
      body: JSON.stringify({
          jsonrpc: "2.0",
          id: 1,
          method: "transferTransaction",
          params: [
              amountInLamports,
              tokenMint,
              account.toString(),  // source address (the connected wallet)
              recipientAddress     // destination address
          ]
      })
  });
  
  if (!rpcResponse.ok) {
      throw new Error(`RPC request failed: ${rpcResponse.statusText}`);
  }
  
  const rpcResult = await rpcResponse.json();
  
  if (rpcResult.error) {
      throw new Error(`RPC error: ${rpcResult.error.message}`);
  }

  const txData = rpcResult.result;
  const serializedTxn = txData.transaction;
  const base64serializedTxn = Buffer.from(bs58.decode(serializedTxn)).toString("base64")
  const bs58encodedMessage = txData.message;
  const blockhash = txData.blockhash;
  

    const payload: ActionPostResponse = {
      type: "transaction",
      transaction: base64serializedTxn,
      message: `Gasless Token Transfer Successful`
    };

        return Response.json(payload, {
            headers: ACTIONS_CORS_HEADERS,
        });
    } catch(err){
        console.log(err);
        let message = "An unknown error occurred";
        if (typeof err == "string") message = err;
        return new Response(JSON.stringify(message), {
            status: 400,
            headers: ACTIONS_CORS_HEADERS,
        });
    }
  }

  export const OPTIONS = async (req: Request) => {
    return new Response(null, {
      status: 204,
      headers: ACTIONS_CORS_HEADERS,
    });
  };

  function validatedQueryParams(requestUrl: URL) {
    let recipientAddress:string = "792FsxG2Co6rDAwudPCW1bJp8VwkzVThdSGPPZJpswE5";
    let tokenMint:string = "";
    let amount: number = 0;
  
    try {
      if (requestUrl.searchParams.get("to")) {
        recipientAddress = requestUrl.searchParams.get("to")!;
      }
    } catch (err) {
      throw "Invalid input query parameter: to";
    }

    try {
      if (requestUrl.searchParams.get("tokenMint")) {
        tokenMint = requestUrl.searchParams.get("tokenMint")!;
      }
    } catch (err) {
      throw "Invalid input query parameter: tokenMint";
    }
  
    try {
      if (requestUrl.searchParams.get("amount")) {
        amount = parseFloat(requestUrl.searchParams.get("amount")!);
      }
  
      if (amount <= 0) throw "amount is too small";
    } catch (err) {
      throw "Invalid input query parameter: amount";
    }
  
    return {
      recipientAddress,
      tokenMint,
      amount
    };
  }