# Adding New Signers to Kora

## Overview

This guide is for wallet service providers who want to integrate their key management solution into Kora. By adding your signer to Kora, you'll enable node operators to use your service for secure transaction signing.

## Architecture Overview

Kora uses an enum-based architecture where all signers are wrapped in a unified `KoraSigner` enum. Your signer will be added as a new variant to this enum, allowing Kora to switch between different signing providers at runtime based on CLI flags.

## Step-by-Step Integration Guide

### Quick Integration Checklist

- [ ] Create your Signer Module
    - Define types and configuration
    - Implement `KoraSigner`'s core signing methods (`sign` and `sign_solana`)
    - Add initialization logic based on your API's requirements
- [ ] Update the `KoraSigner` enum and initialization logic
- [ ] Add CLI arguments
- [ ] Update test scripts to include your signer (see below)
- [ ] Update documentation to include your signer (see below)
- [ ] Submit PR

### Step 1: Create Your Signer Module

Create a new directory under `crates/lib/src/signer/` for your implementation:

```bash
crates/lib/src/signer/
├── your_service/
│   ├── mod.rs      # Module exports
│   ├── signer.rs   # Main implementation
│   ├── config.rs   # Configuration
│   └── types.rs    # Types and configuration
```

Export each of the files in `mod.rs`:

```rust
pub mod config;
pub mod signer;
pub mod types;
```

### Step 2: Define Your Types

In `your_service/types.rs`, define your signer struct and any necessary types (e.g. error types, config, API request/response types, etc.):

```rust
use reqwest::Client;
use serde::{Deserialize, Serialize};
use solana_sdk::pubkey::Pubkey;

#[derive(Clone, Debug)]
pub struct YourServiceSigner {
    // Your API credentials
    pub api_key: String,
    pub api_secret: String,
    pub wallet_id: String,
    
    // HTTP client for API calls
    pub client: Client,
    
    // Cache the public key
    pub public_key: Pubkey,
    
    // Your API base URL
    pub api_base_url: String,
}

// Error types for your signer
#[derive(Debug)]
pub enum YourServiceError {
    MissingConfig(&'static str),
    ApiError(u16),
    InvalidSignature,
    RateLimitExceeded,
    // Add more as needed
}

// Implement Display and Error traits
impl std::fmt::Display for YourServiceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Implementation
    }
}

impl std::error::Error for YourServiceError {}

// API request/response types
#[derive(Serialize)]
pub struct SignTransactionRequest {
    pub method: &'static str,
    pub params: SignTransactionParams,
}

#[derive(Deserialize, Debug)]
pub struct SignTransactionResponse {
    pub method: String,
    pub data: SignTransactionData,
}
```

Consider implementing your error types as a `KoraError` variant in `crates/lib/src/error.rs`.

### Step 3: Implement the Signer Methods

In `your_service/signer.rs`, implement the core signing logic. Some methods you should implement are:

- `new`: Create a new instance of your signer
- `solana_pubkey`: Get the Solana public key for this signer (as a Solana `Pubkey`)
- `sign`: Sign a `VersionedTransaction` and return `Vec<u8>` (raw bytes)
- `sign_solana`: Sign a `VersionedTransaction` and return a Solana `Signature`
- other methods core to your signer's implementation (e.g., `init`, `call_signing_api`, etc.)

```rust
use crate::signer::Signature as KoraSignature;
use solana_sdk::{
    signature::Signature,
    transaction::VersionedTransaction,
};

impl YourServiceSigner {
    /// Create a new instance of your signer
    pub fn new(
        api_key: String,
        api_secret: String,
        wallet_id: String,
    ) -> Result<Self, YourServiceError> {
        Ok(Self {
            api_key,
            api_secret,
            wallet_id,
            client: reqwest::Client::new(),
            public_key: Pubkey::default(), // Will be initialized later
            api_base_url: "https://api.yourservice.com/v1".to_string(),
        })
    }
    
    /// Get the Solana public key for this signer
    pub fn solana_pubkey(&self) -> Pubkey {
        self.public_key
    }
       
    /// Sign a transaction and return raw bytes
    pub async fn sign(
        &self,
        transaction: &VersionedTransaction,
    ) -> Result<Vec<u8>, YourServiceError> {
        // 1. Serialize the transaction message
        let message_bytes = transaction.message.serialize();
        
        // 2. Call your API to sign
        let signature = self.call_signing_api(&message_bytes).await?;
        
        // 3. Return the signature bytes
        Ok(signature)
    }
    
    /// Sign and return a Solana signature
    pub async fn sign_solana(
        &self,
        transaction: &VersionedTransaction,
    ) -> Result<Signature, YourServiceError> {
        let sig_bytes = self.sign(transaction).await?;
        
        // Convert to Solana signature (must be exactly 64 bytes)
        let sig_array: [u8; 64] = sig_bytes
            .try_into()
            .map_err(|_| YourServiceError::InvalidSignature)?;
            
        Ok(Signature::from(sig_array))
    }
    
    // Private helper methods    
    async fn call_signing_api(
        &self,
        message: &[u8],
    ) -> Result<Vec<u8>, YourServiceError> {
        // Implementation specific to your API
    }
}
```

### Step 4: Update the KoraSigner Enum

Add your signer to the `KoraSigner` enum in `crates/lib/src/signer/signer.rs`:

```rust
#[derive(Clone)]
pub enum KoraSigner {
    Memory(SolanaMemorySigner),
    Turnkey(TurnkeySigner),
    Vault(VaultSigner),
    Privy(PrivySigner),
    YourService(YourServiceSigner), // Add your signer here
}

// Update the trait implementation
impl KoraSigner {
    pub fn solana_pubkey(&self) -> Pubkey {
        match self {
            // ... existing implementations
            KoraSigner::YourService(signer) => signer.solana_pubkey(),
        }
    }
}

impl super::Signer for KoraSigner {
    type Error = KoraError;
    
    async fn sign(
        &self,
        transaction: &VersionedTransaction,
    ) -> Result<super::Signature, Self::Error> {
        match self {
            // ... existing implementations
            KoraSigner::YourService(signer) => {
                let sig = signer.sign(transaction).await?;
                Ok(super::Signature {
                    bytes: sig,
                    is_partial: false,
                })
            }
        }
    }
    
    async fn sign_solana(
        &self,
        transaction: &VersionedTransaction,
    ) -> Result<SolanaSignature, Self::Error> {
        match self {
            // ... existing implementations
            KoraSigner::YourService(signer) => {
                signer.sign_solana(transaction)
                    .await
                    .map_err(KoraError::from)
            }
        }
    }
}
```

### Step 5: Add Initialization Logic

Update `crates/lib/src/signer/init.rs` to include your signer:

- add `init_your_service_signer` to define your service's signer as a `KoraSigner`
- add your service to the `init_signer_type` function if specified in the CLI args (we'll be adding these in the next step)

```rust
// Add your args struct import
use crate::rpc_server::args::YourServiceArgs;

pub fn init_signer_type(args: &RpcArgs) -> Result<KoraSigner, KoraError> {
    if args.turnkey_args.turnkey_signer {
        init_turnkey_signer(&args.turnkey_args)
    } else if args.vault_args.vault_signer {
        init_vault_signer(&args.vault_args)
    } else if args.privy_args.privy_signer {
        init_privy_signer(&args.privy_args)
    } else if args.your_service_args.your_service_signer {
        init_your_service_signer(&args.your_service_args)
    } else {
        init_memory_signer(args.private_key.as_ref())
    }
}

fn init_your_service_signer(config: &YourServiceArgs) -> Result<KoraSigner, KoraError> {
    // Extract required configuration
    let api_key = config
        .your_service_api_key
        .clone()
        .or_else(|| std::env::var("YOUR_SERVICE_API_KEY").ok())
        .ok_or_else(|| KoraError::SigningError("YourService API key required".to_string()))?;
        
    let api_secret = config
        .your_service_api_secret
        .clone()
        .or_else(|| std::env::var("YOUR_SERVICE_API_SECRET").ok())
        .ok_or_else(|| KoraError::SigningError("YourService API secret required".to_string()))?;
        
    let wallet_id = config
        .your_service_wallet_id
        .clone()
        .or_else(|| std::env::var("YOUR_SERVICE_WALLET_ID").ok())
        .ok_or_else(|| KoraError::SigningError("YourService wallet ID required".to_string()))?;
        
    // Create the signer
    let mut signer = YourServiceSigner::new(api_key, api_secret, wallet_id)?;
    
    // Initialize if needed (fetch public key, etc)
    // Note: This would need to be handled async in practice
    
    Ok(KoraSigner::YourService(signer))
}
```

### Step 6: Add CLI Arguments

Update `crates/lib/src/rpc_server/args.rs` to add your service's configuration:

- Define your service's arguments struct (e.g. `YourServiceArgs`). The struct must include a `your_service_signer` field (boolean type) that is used to determine if the signer should be used. Make sure to add the `#[clap(long = "with-your-service-signer", help_heading = "YourService Signer")]` attribute to the field to define the flag and create a help menu entry for your service. Add any fields that are required for a user to connect with your service.
- Add your service to the `RpcArgs` struct using `#[command(flatten)]` to include your service's arguments struct in the `RpcArgs` struct. Make sure to add your service's signer flag to the `required_unless_present_any` array in the `private_key` field.

```rust
#[derive(clap::Args, Debug, Clone)]
pub struct YourServiceArgs {
    /// Use YourService signer for transaction signing
    #[clap(long = "with-your-service-signer", help_heading = "YourService Signer")]
    pub your_service_signer: bool,
    
    /// YourService API key
    #[clap(long, env = "YOUR_SERVICE_API_KEY", help_heading = "YourService Signer")]
    pub your_service_api_key: Option<String>,
    
    /// YourService API secret
    #[clap(long, env = "YOUR_SERVICE_API_SECRET", help_heading = "YourService Signer")]
    pub your_service_api_secret: Option<String>,
    
    /// YourService wallet ID
    #[clap(long, env = "YOUR_SERVICE_WALLET_ID", help_heading = "YourService Signer")]
    pub your_service_wallet_id: Option<String>,
}

// Add to RpcArgs struct
#[derive(clap::Args, Debug, Clone)]
pub struct RpcArgs {
    #[arg(long, env = "KORA_PRIVATE_KEY", required_unless_present_any = [/*existing signer flags*/, "with-your-service-signer"], help_heading = "Signing Options")]
    pub private_key: Option<String>,

    // ... existing fields
    
    #[clap(flatten)]
    pub your_service_args: YourServiceArgs,
}
```

### Step 7: Export Your Module

Update `crates/lib/src/signer/mod.rs`:

```rust
pub mod your_service;
// ... other modules

pub use your_service::types::YourServiceSigner;
```

## Testing Your Integration

### Environment Variables

Add the same environment variables you defined in your CLI arguments to the following files: `.env.example` and `.env` in the root of the project, and in `./sdks/ts/`:

- `YOUR_SERVICE_API_KEY`: The API key for your service.
- `YOUR_SERVICE_API_SECRET`: The API secret for your service.
- `YOUR_SERVICE_WALLET_ID`: The wallet ID for your service.

### Integration Tests


Testing should utilize the existing `sdks/ts/test/integration.test.ts` file. We manage this with an environment variable `KORA_SIGNER_TYPE` that is set to the signer type you are testing.

Setup:
- `sdks/ts/test/setup.ts`: You will need to add an environment variable to `loadEnvironmentVariables`  to set the signer type to your service.
- `sdks/ts/package.json`: You will need to add a new test script that uses the signer type you added to the setup script: `"test:integration:your-service": "KORA_SIGNER_TYPE=your-service pnpm test integration.test.ts"`. When executed, the test will run with the signer type set to your service.

Integration testing requires a local Solana test validator and a local Kora node. We can use the [TypeScript Test Makefile](/makefiles/tests_ts.mk) to start a local Solana test validator and a local Kora node by adding a new script for your signer:

```makefile
# Run TypeScript tests with YourService signer  
test-ts-integration-your-service:
	@$(call start_solana_validator)
	@echo "🚀 Starting Kora node with YourService signer..."
	@$(call stop_kora_server)
	@cargo run -p kora-cli --bin kora -- --config $(REGULAR_CONFIG) --rpc-url $(TEST_RPC_URL) rpc --with-your-service-signer --port $(TEST_PORT) $(QUIET_OUTPUT) &
	@echo $$! > .kora.pid
	@echo "⏳ Waiting for server to start..."
	@sleep 5
	@printf "Running TypeScript SDK tests with YourService signer...\n"
	-@cd sdks/ts && pnpm test:integration:your-service
	@$(call stop_kora_server)
	@$(call stop_solana_validator)
```

And add your new script to the `test-ts-signers` script:

```makefile
test-ts-signers: test-ts-integration-your-service # ... test-ts-existing-signer-tests
```

Make sure your local environment is set up:

```makefile
make install
make install-ts-sdk
make build-ts-sdk
```

Now you can test your integration by running:

```bash
make test-ts-integration-your-service
```


## Documentation Requirements

When submitting your PR, include:

### 1. Update the Signers Guide


Add a section to [`docs/operators/SIGNERS.md`](/docs/operators/SIGNERS.md) for your signer explaining the prerequisites, setup, and usage.

```markdown
## YourService Signer

[YourService](https://yourservice.com) provides [brief description of your service].

### Prerequisites

- YourService account
- API credentials
- Funded wallet

### Setup

1. Get your API credentials from [dashboard link]
2. Create a wallet...
3. Configure environment variables:

\```bash
YOUR_SERVICE_API_KEY="your_api_key"
YOUR_SERVICE_API_SECRET="your_api_secret"
YOUR_SERVICE_WALLET_ID="your_wallet_id"
\```

### Usage

\```bash
kora rpc --with-your-service-signer
\```
```

### 2. Update README

Add your service to the main README's signer list.

### 3. Add Example Configuration

Create an example `.env` configuration:

```bash
# YourService Signer Configuration
YOUR_SERVICE_API_KEY=your_api_key_here
YOUR_SERVICE_API_SECRET=your_api_secret_here
YOUR_SERVICE_WALLET_ID=your_wallet_id_here
```


## Submission Checklist

Before submitting your PR:

- [ ] Code compiles without warnings
- [ ] All tests pass
- [ ] Documentation is complete
- [ ] Example configuration (.env.example) provided
- [ ] No hardcoded values or secrets
- [ ] Error messages are helpful
- [ ] Follows Rust naming conventions (snake_case)
- [ ] Linting passes (`make lint` and `make format-ts-sdk`)
- [ ] Contact the Kora team with API Keys for integration testing

## Getting Help

- Open an issue for design discussions
- Join our community channels
- Review existing signer implementations for patterns in [`crates/lib/src/signer/`](/crates/lib/src/signer/)

## Example PR Structure

```sh
feat(signer): add YourService signer integration

- Implement Signer trait for YourService
- Add CLI arguments and initialization
- Add signer to Signers Guide
- Add integration tests script to Makefile

```

Welcome to the Kora ecosystem! We're excited to have your key management solution as part of the platform.
