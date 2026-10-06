import { config } from "dotenv";
import express, { Request, Response } from "express";
import {
    type PaymentRequirements,
    type PaymentPayload,
    type Network,
    type VerifyResponse,
    type SettleResponse,
} from "@x402/core/types";
import { x402Facilitator } from "@x402/core/facilitator";
import {
    SOLANA_DEVNET_CAIP2,
    TransactionOnchainFailureError,
    createRpcClient,
    type FacilitatorSvmSigner,
} from "@x402/svm";
import { registerExactSvmScheme } from "@x402/svm/exact/facilitator";
import { address, signature, type Base64EncodedWireTransaction } from "@solana/kit";
import { KoraClient } from "@solana/kora";
import path from "path";

config({ path: path.join(process.cwd(), '..', '.env') });

const KORA_RPC_URL = process.env.KORA_RPC_URL || "http://localhost:8080/";
const SOLANA_RPC_URL = process.env.SOLANA_RPC_URL;
const FACILITATOR_PORT = process.env.FACILITATOR_PORT || 3000;
const NETWORK = (process.env.NETWORK || SOLANA_DEVNET_CAIP2) as Network;
const KORA_API_KEY = process.env.KORA_API_KEY || "kora_facilitator_api_key_example";
const CONFIRM_TIMEOUT_MS = 30_000;
const CONFIRM_POLL_MS = 1_000;

const kora = new KoraClient({ rpcUrl: KORA_RPC_URL, apiKey: KORA_API_KEY });
const rpc = createRpcClient(NETWORK, SOLANA_RPC_URL);

async function createKoraSigner(): Promise<FacilitatorSvmSigner> {
    const { signer_address } = await kora.getPayerSigner();
    const feePayer = address(signer_address);

    return {
        getAddresses: () => [feePayer],

        signTransaction: async (transaction) => {
            const { signed_transaction } = await kora.signTransaction({ transaction });
            return signed_transaction;
        },

        simulateTransaction: async (transaction) => {
            await kora.signTransaction({ transaction });
        },

        sendTransaction: async (transaction) => {
            return await rpc
                .sendTransaction(transaction as Base64EncodedWireTransaction, { encoding: "base64" })
                .send();
        },

        confirmTransaction: async (txSignature) => {
            const deadline = Date.now() + CONFIRM_TIMEOUT_MS;
            while (Date.now() < deadline) {
                const { value: [status] } = await rpc.getSignatureStatuses([signature(txSignature)]).send();
                if (status?.confirmationStatus === "confirmed" || status?.confirmationStatus === "finalized") {
                    if (status.err) {
                        throw new TransactionOnchainFailureError(`Transaction failed onchain: ${JSON.stringify(status.err)}`);
                    }
                    return { slot: status.slot };
                }
                await new Promise((resolve) => setTimeout(resolve, CONFIRM_POLL_MS));
            }
            throw new Error("Transaction confirmation timeout");
        },
    };
}

async function main() {
    const facilitator = registerExactSvmScheme(new x402Facilitator(), {
        signer: await createKoraSigner(),
        networks: NETWORK,
    });

    const app = express();

    app.use(express.json());

    app.get("/verify", (req: Request, res: Response) => {
        res.json({
            endpoint: "/verify",
            description: "POST to verify x402 payments",
            body: {
                paymentPayload: "PaymentPayload",
                paymentRequirements: "PaymentRequirements",
            },
        });
    });

    app.post("/verify", async (req: Request, res: Response) => {
        console.log("=== /verify endpoint called ===");
        try {
            const { paymentPayload, paymentRequirements } = req.body as {
                paymentPayload: PaymentPayload;
                paymentRequirements: PaymentRequirements;
            };

            const verifyResponse = await facilitator.verify(paymentPayload, paymentRequirements);
            res.status(verifyResponse.isValid ? 200 : 400).json(verifyResponse);
        } catch (error) {
            const verifyResponse: VerifyResponse = {
                isValid: false,
                invalidReason: error instanceof Error ? error.message : "Verification failed",
            };
            res.status(400).json(verifyResponse);
        }
    });

    app.get("/settle", (req: Request, res: Response) => {
        res.json({
            endpoint: "/settle",
            description: "POST to settle x402 payments",
            body: {
                paymentPayload: "PaymentPayload",
                paymentRequirements: "PaymentRequirements",
            },
        });
    });

    app.get("/supported", (req: Request, res: Response) => {
        console.log("=== /supported endpoint called ===");
        res.json(facilitator.getSupported());
    });

    app.post("/settle", async (req: Request, res: Response) => {
        console.log("=== /settle endpoint called ===");
        try {
            const { paymentPayload, paymentRequirements } = req.body as {
                paymentPayload: PaymentPayload;
                paymentRequirements: PaymentRequirements;
            };

            const response = await facilitator.settle(paymentPayload, paymentRequirements);
            res.status(response.success ? 200 : 400).json(response);
        } catch (error) {
            const response: SettleResponse = {
                transaction: "",
                success: false,
                network: NETWORK,
                errorReason: error instanceof Error ? error.message : "Settlement failed",
            };
            res.status(400).json(response);
        }
    });

    app.listen(FACILITATOR_PORT, () => {
        console.log(`Server listening at http://localhost:${FACILITATOR_PORT}`);
    });
}

main().catch((error) => {
    console.error("Failed to start facilitator:", error);
    process.exit(1);
});
