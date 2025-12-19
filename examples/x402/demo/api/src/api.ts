import express from "express";
import { Network, paymentMiddleware } from "@x402/express";
import { HTTPFacilitatorClient, x402ResourceServer } from "@x402/core/server";
import { ExactSvmScheme } from "@x402/svm/exact/server";
import { SOLANA_DEVNET_CAIP2 } from "@x402/svm";
import { Address } from "@solana/addresses";
import { config } from "dotenv";
import path from "path";

config({ path: path.join(process.cwd(), '..', '.env') });

type Resource = `${string}://${string}`;

const API_PORT = process.env.API_PORT || 4021;
const FACILITATOR_URL = process.env.FACILITATOR_URL as Resource || "http://localhost:3000";
const NETWORK = (process.env.NETWORK || SOLANA_DEVNET_CAIP2) as Network;
const KORA_SIGNER_ADDRESS = process.env.KORA_SIGNER_ADDRESS as Address;

if (!KORA_SIGNER_ADDRESS) {
    throw new Error("KORA_SIGNER_ADDRESS is not set");
}
if (!FACILITATOR_URL) {
    console.error("❌ FACILITATOR_URL environment variable is required");
    process.exit(1);
}
const facilitatorClient = new HTTPFacilitatorClient({ url: FACILITATOR_URL });


const app = express();

app.use(
    paymentMiddleware(
        {
            "GET /protected": {
                accepts: [
                    {
                      scheme: "exact",
                      price: "$0.001",
                      network: NETWORK,
                      payTo: KORA_SIGNER_ADDRESS,
                    },
                  ],
                description: "Protected endpoint",
                mimeType: "application/json",
            },
        },
        new x402ResourceServer(facilitatorClient)
            .register(NETWORK, new ExactSvmScheme()),
    ),
);

app.get("/protected", (req, res) => {
    res.json({
        message: "Protected endpoint accessed successfully",
        timestamp: new Date().toISOString(),
    });
});

app.get("/health", (req, res) => {
    res.json({ status: "ok" });
});


app.listen(API_PORT, () => {
    console.log(`Server listening at http://localhost:${API_PORT}`);
});

// curl -X GET http://localhost:4021/protected
// curl -X GET http://localhost:4021/health