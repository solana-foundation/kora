## Jito Bundles with Kora

This sample code will help you get started with Jito Bundles with Kora.

## Full Demo

A full demo of Kora is available [here](https://launch.solana.com/docs/kora/guides/jito-demo).

## Sample Output

```bash
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
KORA JITO BUNDLE DEMO
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

[1/5] Initializing clients
  → Kora RPC: http://localhost:8080/
  → Solana RPC: https://api.mainnet-beta.solana.com

[2/5] Setting up keypairs
  → Sender: BYJVBqQ2xV9GECc84FeoPQy2DpgoonZQFQu97MMWTbBc
  → Kora signer address: 3Z1Ef7YaxK8oUMoi6exf7wYZjZKWJJsrzJXSt1c3qrDE

[3/5] Creating bundle transactions
  → Blockhash: 7HZUaMqV...
  → Tip account: 96gYZGLn...
  → Transaction 1: Kora Memo "Bundle tx #1"
  → Transaction 2: Kora Memo "Bundle tx #2"
  → Transaction 3: Kora Memo "Bundle tx #3"
  → Transaction 4: Kora Memo "Bundle tx #4" + Jito tip
  ✓ 4 transactions created for bundle

[4/5] Signing bundle with Kora
  ✓ All transactions signed by user
  ✓ Bundle co-signed by Kora
  → 4 transactions signed

[5/5] Submitting bundle to Jito
  ✓ Bundle submitted to Jito block engine
  → Bundle UUID: 8f4a3b2c-1d5e-6f7a-8b9c-0d1e2f3a4b5c
  ⏳ Polling bundle status...
  ✓ Bundle landed (simulated for demo)

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
SUCCESS: Bundle confirmed on Solana
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

Bundle UUID:
8f4a3b2c-1d5e-6f7a-8b9c-0d1e2f3a4b5c
```