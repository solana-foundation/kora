import { config } from "dotenv";
import express, { Request, Response } from "express";
import {
    PaymentRequirementsSchema,
    type PaymentRequirements,
    type PaymentPayload,
    PaymentPayloadSchema,
    SupportedSVMNetworks,
    ExactSvmPayload,
    SettleResponse,
    Network,
} from "x402/types";
import { KoraClient } from "@kora/sdk";
import path from "path";

config({ path: path.join(process.cwd(), '..', '.env') });

const KORA_RPC_URL = process.env.KORA_RPC_URL || "http://localhost:8080/";
const FACILITATOR_PORT = process.env.FACILITATOR_PORT || 3000;
const NETWORK = (process.env.NETWORK || "solana-devnet") as Network;
const KORA_API_KEY = process.env.KORA_API_KEY || "kora_facilitator_api_key_example";

const app = express();

app.use(express.json());

type VerifyRequest = {
    paymentPayload: PaymentPayload;
    paymentRequirements: PaymentRequirements;
};

type SettleRequest = {
    paymentPayload: PaymentPayload;
    paymentRequirements: PaymentRequirements;
};

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

    const kora = new KoraClient({ rpcUrl: KORA_RPC_URL, apiKey: KORA_API_KEY });

    try {
        const body: VerifyRequest = req.body;
        const paymentRequirements = PaymentRequirementsSchema.parse(body.paymentRequirements);

        const paymentPayload = PaymentPayloadSchema.parse(body.paymentPayload);

        if (!SupportedSVMNetworks.includes(paymentRequirements.network)) {
            throw new Error("Invalid network");
        }
        const { transaction } = paymentPayload.payload as ExactSvmPayload;

        const { signed_transaction } = await kora.signTransaction({
            transaction
        });

        const valid = !!signed_transaction;

        const verifyResponse = {
            isValid: valid,
        };

        res.json(verifyResponse);
    } catch (error) {
        res.status(400).json({ error: "Invalid request" });
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

app.get("/supported", async (req: Request, res: Response) => {
    console.log("=== /supported endpoint called ===");
    try {
        const kora = new KoraClient({ rpcUrl: KORA_RPC_URL, apiKey: KORA_API_KEY });

        const { signer_address } = await kora.getPayerSigner();

        const kinds = [{
            x402Version: 1,
            scheme: "exact",
            network: NETWORK,
            extra: {
                feePayer: signer_address,
            },
        }];

        res.json({
            kinds,
        });
    } catch (error) {
        res.status(500).json({
            error: `Failed to get supported payment kinds: ${error instanceof Error ? error.message : String(error)}`
        });
    }
});

app.post("/settle", async (req: Request, res: Response) => {
    console.log("=== /settle endpoint called ===");
    try {
        const body: SettleRequest = req.body;
        const paymentRequirements = PaymentRequirementsSchema.parse(body.paymentRequirements);
        const paymentPayload = PaymentPayloadSchema.parse(body.paymentPayload);

        if (!SupportedSVMNetworks.includes(paymentRequirements.network)) {
            throw new Error("Invalid network");
        }
        const { transaction } = paymentPayload.payload as ExactSvmPayload;

        const kora = new KoraClient({ rpcUrl: KORA_RPC_URL, apiKey: KORA_API_KEY });
        const { signature } = await kora.signAndSendTransaction({
            transaction
        });

        const response: SettleResponse = {
            transaction: signature,
            success: true,
            network: NETWORK,
        }

        res.json(response);
    } catch (error) {
        res.status(400).json({ error: `Invalid request: ${error}` });
    }
});

app.listen(FACILITATOR_PORT, () => {
    console.log(`Server listening at http://localhost:${FACILITATOR_PORT}`);
});