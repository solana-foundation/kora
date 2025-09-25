import { config } from "dotenv";
import { createSigner, decodeXPaymentResponse, wrapFetchWithPayment, MultiNetworkSigner } from "x402-fetch";
import path from "path";

config({ path: path.join(process.cwd(), '..', '.env') });

const PAYER_PRIVATE_KEY = process.env.PAYER_PRIVATE_KEY as string;
const PROTECTED_API_URL = process.env.PROTECTED_API_URL || "http://localhost:4021/protected";
const NETWORK =  process.env.NETWORK || "solana-devnet";

async function main() {
  console.log('\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━');
  console.log('X402 + KORA PAYMENT FLOW DEMONSTRATION');
  console.log('━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━');

  if (!PAYER_PRIVATE_KEY) {
    console.error("\n❌ ERROR: Missing required environment variables");
    console.error("  → Please ensure TEST_SENDER_KEYPAIR is set in your .env file");
    process.exit(1);
  }

  try {
    console.log('\n[1/4] Initializing payment signer');
    const payer = await createSigner(NETWORK, PAYER_PRIVATE_KEY);
    const signer = { svm: payer } as MultiNetworkSigner;
    console.log('  → Network:', NETWORK);
    console.log('  → Payer address:', signer.svm.address.slice(0, 4) + '...' + signer.svm.address.slice(-4));
    console.log('  ✓ Signer initialized');
    const fetchWithPayment = wrapFetchWithPayment(fetch, payer);

    console.log('\n[2/4] Attempting to access protected endpoint without payment');
    console.log('  → GET', PROTECTED_API_URL);
    const expect402Response = await fetch(PROTECTED_API_URL, {
      method: "GET",
    });
    console.log('  → Response:', expect402Response.status, expect402Response.statusText);
    console.log(`  ${expect402Response.status === 402 ? "✅" : "❌"} Status code: ${expect402Response.status}`);

    console.log('\n[3/4] Accessing protected endpoint with x402 payment');
    console.log('  → Using x402 fetch wrapper');
    console.log('  → Payment will be processed via Kora facilitator');
    const response = await fetchWithPayment(PROTECTED_API_URL, {
      method: "GET",
    });
    console.log('  → Transaction submitted to Solana');
    console.log(`  ${response.status === 200 ? "✅" : "❌"} Status code: ${response.status}`);

    console.log('\n[4/4] Processing response data');
    const data = await response.json();
    const paymentResponseHeader = response.headers.get("x-payment-response");

    let decodedPaymentResponse;
    try {
      decodedPaymentResponse = decodeXPaymentResponse(paymentResponseHeader!);
      console.log('  ✓ Payment response decoded');
    } catch (decodeError) {
      decodedPaymentResponse = null;
      console.log('  ⚠ No payment response to decode');
    }

    const result = {
      data: data,
      status_code: response.status,
      payment_response: decodedPaymentResponse
    };

    console.log('\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━');
    console.log('SUCCESS: Payment completed and API accessed');
    console.log('━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━');

    console.log('\nResponse Data:');
    console.log(JSON.stringify(result, null, 2));

    if (decodedPaymentResponse?.transaction) {
      console.log('\nTransaction signature:');
      console.log(decodedPaymentResponse.transaction);
      console.log('\nView on explorer:');
      const explorerUrl = NETWORK === 'solana-devnet'
        ? `https://explorer.solana.com/tx/${decodedPaymentResponse.transaction}?cluster=devnet`
        : `https://explorer.solana.com/tx/${decodedPaymentResponse.transaction}?cluster=custom&customUrl=http%3A%2F%2Flocalhost%3A8899`;
      console.log(explorerUrl);
    }

    process.exit(0);
  } catch (error) {
    console.log('\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━');
    console.log('ERROR: Demo failed');
    console.log('━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━');

    const errorResult = {
      success: false,
      error: error instanceof Error ? error.message : String(error),
      status_code: (error as any).response?.status
    };

    console.log('\nError details:');
    console.log(JSON.stringify(errorResult, null, 2));

    console.log('\nTroubleshooting tips:');
    console.log('  → Ensure all services are running (Kora, Facilitator, API)');
    console.log('  → Verify your account has sufficient USDC balance');
    console.log('  → Check that Kora fee payer has SOL for gas');
    console.log('  → Confirm API key matches in .env and kora.toml');

    process.exit(1);
  }
}
main();
