# Kora Quick Start Guide

## Kora Basics

Kora is a JSON-RPC server that provides fee payment services for Solana transactions. It allows users to pay transaction fees with SPL tokens instead of SOL, enabling better UX for applications where users may not hold SOL.

Kora RPC validates client requests based on a configuration (`kora.toml`) that defines allowable programs, wallets, tokens, etc. Once validated, the Kora server will sign the transaction and send it to the network (or return a serialized siged transaction to the client).

```
┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
│   Client App    │───▶│   Kora RPC      │───▶│   Solana RPC    │
│                 │    │   Server        │    │                 │
└─────────────────┘    └─────────────────┘    └─────────────────┘
                              │
                              ▼
                       ┌──────────────────────┐
                       │   Kora Private Key   │
                       │ ( or Turnkey/Privy ) │
                       └──────────────────────┘
```

This quick start will launch a local Kora RPC server and demonstrate client integration for testing fee payment workflows.

## Requirements

- [Solana CLI](https://solana.com/docs/intro/installation) v2.2.x or greater
- [Rust/Cargo](https://doc.rust-lang.org/cargo/getting-started/installation.html) (for Kora RPC installation)
- [Node.js v24+](https://nodejs.org/en/download) and a package manager (e.g., [pnpm](https://pnpm.io/), npm)

## Install Kora RPC

Install the Kora RPC server globally:

```bash
cargo install kora-cli
```

## Create Project

Clone the repository and navigate to the getting started demo directory:

```bash
git clone https://github.com/solana-foundation/kora
cd kora/docs/getting-started/demo
```

### Project Structure

The demo contains three main components:

**Client Directory (`client/src/`)**
- `setup.ts` - Local environment setup (creates keypairs & writes them to .env, airdrops SOL, initializes test token)
- `types.ts` - TypeScript definitions for Kora RPC request/response types
- `client.ts` - Kora client factory using Solana Kit
- `main.ts` - Main demonstration script showing Kora integration

**Server Directory (`server/`)**
- `kora.toml` - Kora RPC configuration defining validation rules, allowed tokens, and fee parameters

**Shared Configuration**
- `.env` - Environment variables for keypairs and addresses (create `.env` in root - `demo/.env`). The environment variables will be created by the setup script.

## Setup Environment

First, create .env for your environment :

```bash
touch .env
```

### Setup Client

Install client dependencies:

```bash
cd client
pnpm install  # or npm install
```
### Setup Kora RPC Server

The Kora server requires configuration to specify which tokens can be used for fee payment. Open `server/kora.toml` and note the validation section. Here we can specify several parameters that will be validated prior to signing a transaction:

- `max_allowed_lamports`: maximum transaction fee you are willing to pay on behalf of the user
- `max_signatures`: maximum number of signatures a transaction can have
- `price_source`: oracle for determining token price ("Mock" or "Jupiter")
- `allowed_programs`: whitelist of program IDs that can be executed (e.g., System Program, Token Program)
- `allowed_tokens`: whitelist of tokens that are allowed to be transferred
- `allowed_spl_paid_tokens`: array of mint addresses your program accepts as payment
- `disallowed_accounts`: blacklist of accounts not allowed to interact with your kora RPC

For now, let's leave the default values--you can come back here and change these later. 

## Test Server

Open three terminals and run the following commands:

### Terminal 1: Start Local Test Validator
```bash
# From project root or anywhere
solana-test-validator -r
```

### Terminal 2: Initialize Environment
```bash
# From ./client directory
pnpm init-env
```

This script will:
- Generate keypairs and save them to `.env`
- Airdrop SOL to test accounts
- Create and initialize a local USDC token
- Fund test accounts with tokens

**Important** Make sure you copy the public key of the new USDC test token from your .env and update the `allowed_tokens` and `allowed_spl_paid_tokens` in `./server/kora.toml`.

```toml
allowed_tokens = [
    "YOUR_USDC_LOCAL_PUBLICK_KEY"    # Update this based on the USDC_LOCAL_KEY public key comment in your .env
]
allowed_spl_paid_tokens = [
    "YOUR_USDC_LOCAL_PUBLICK_KEY"    # Update this based on the USDC_LOCAL_KEY public key comment in your .env
] 
```

### Terminal 3: Start Kora RPC Server
```bash
# From ./server directory
kora rpc
```

The server reads configuration from `kora.toml` and uses environment variables from the shared `.env` file. If you are using a different folder structure than specified here, you may need to use the `--config` to specify the location of `kora.toml` and `--private-key` to specify the directory of your private key. You can access `kora rpc -h` for help on the RPC server options.

### Terminal 4: Run Client Demo
```bash
# From ./client directory
pnpm start
```

You should see output similar to:

```bash
Kora Config: {
  fee_payer: 'Df2UmGQH86TBDsub7XZoSAo7KZa1ZJZr2w1PL1APUjjU',
  validation_config: {
    max_allowed_lamports: 1000000,
    max_signatures: 10,
    allowed_programs: [
      '11111111111111111111111111111111',
      'TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA',
      'ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL'
    ],
    allowed_tokens: [
      'usdCAEFbouFGxdkbHCRtMTcN7DJHd3aCmP9vqjLgmAp' 
    ],
    allowed_spl_paid_tokens: [
      'usdCAEFbouFGxdkbHCRtMTcN7DJHd3aCmP9vqjLgmAp' 
    ],
    disallowed_accounts: [],
    price_source: 'Mock'
  }
}
Blockhash:  C8W8d5w2H4jKXyFg5CEBoiaPvEpJ1am7xLxZ3fym4a2g
```

This confirms your Kora server is running and properly configured!

## Next Steps

Once you have the basic setup working, explore additional Kora RPC methods:

- `estimateTransactionFee` - Calculate fees for transactions
- `getSupportedTokens` - Returns an array of supported payment tokens
- `signTransaction` - Sign transactions with the Kora feepayer
- `transferTransaction` - Create transfer SOL or SPL token transfer transactions (signed by the Kora feepayer)
- `signAndSendTransaction` - Signs a transaction with the Kora feepayer and sends it to the configured Solana RPC
- `signTransactionIfPaid` - Conditionally sign transactions when fees are covered

**Got questions?** Ask questions the [Solana Stack Exchange](https://solana.stackexchange.com/) with a `Kora` tag.