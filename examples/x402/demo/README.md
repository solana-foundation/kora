# Solana x402 Protocol Integration with Kora RPC

*Updated December 18, 2025 to x402 v2 spec*

## What You'll Build

This guide walks you through implementing a complete x402 (HTTP 402 Payment Required) integration with Kora, Solana gasless signing infrastructure. By the end, you'll have a working system where:

- APIs can charge micropayments for access using the x402 protocol
- Users pay in USDC without needing SOL for gas fees
- Kora handles all transaction fees as the gasless facilitator
- Payments are settled atomically on Solana blockchain

The final result will be a fully functional payment-protected API:

```shell
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
X402 + KORA PAYMENT FLOW DEMONSTRATION
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

[1/4] Initializing payment signer
  → Network: solana-devnet
  → Payer address: BYJV...TbBc
  ✓ Signer initialized

[2/4] Attempting to access protected endpoint without payment
  → GET http://localhost:4021/protected
  → Response: 402 Payment Required
  ✅ Status code: 402

[3/4] Accessing protected endpoint with x402 payment
  → Using x402 fetch wrapper
  → Payment will be processed via Kora facilitator
  → Transaction submitted to Solana
  ✅ Status code: 200

[4/4] Processing response data
  ✓ Payment response decoded

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
SUCCESS: Payment completed and API accessed
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

Response Data:
{
    "data": {
        "message": "Protected endpoint accessed successfully",
        "timestamp": "2025-09-25T20:14:04.242Z"
    },
    "status_code": 200,
    "payment_response": {
        "transaction": "5ULZpdeThaMAy6hcEGfAoMFqJqPpCtxdCxb6JYUV6nA4x8Lk2hKEuzofGUPoe1pop6BdWMSmF5oRPrXsbdWmpruf",
        "success": true,
        "network": "solana-devnet"
    }
}
```

Full Guide available [here](https://launch.solana.com/docs/kora/guides/x402).

## What is x402?

[x402](https://www.x402.org/) is an open payment standard that enables seamless micropayments for API access. Instead of traditional subscription models or API keys, x402 allows servers to charge for individual API calls, creating true pay-per-use infrastructure.

Key benefits of x402:
- **Instant Micropayments**: Pay fractions of a cent per API call
- **Enable AI agents to pay for API calls**: Pay for API calls with AI agents
- **No Subscriptions**: Users only pay for what they use
- **Web3 Payments**: Transparent, verifiable payments on-chain
- **Standard HTTP**: Works with existing web infrastructure using an HTTP 402 status code when payment is required

Servers using x402 to require micropayments for API access will return an HTTP 402 status code when payment is required. To access protected endpoints, clients must pass a valid payment to the server in a `X-PAYMENT` header. x402 relies on "Facilitators" to verify and settle transactions so that servers don't need to directly interact with blockchain infrastructure.

### Understanding Facilitators

Facilitators are a crucial component in the x402 ecosystem. They act as specialized services that abstract blockchain payments on behalf of API servers.

**What Facilitators Do:**
- **Verify Payments**: Validate that client's payment payloads are correctly formed and sufficient
- **Abstract Complexity**: Remove the need for servers to directly interact with blockchain infrastructure (signing and paying network fees)
- **Settle Transactions**: Submit validated transactions to Solana (or other networks)

