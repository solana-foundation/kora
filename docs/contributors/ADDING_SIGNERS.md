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
- [ ] Update the `KoraSigner` enum
- [ ] Update `SignerTypeConfig` enum in `config.rs` for multi-signer support
- [ ] Add initialization logic in `init.rs`
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

### Step 5: Update the SignerTypeConfig Enum

Add your signer to the `SignerTypeConfig` enum in `crates/lib/src/signer/config.rs`:

```rust
/// Signer type-specific configuration with environment variable references
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SignerTypeConfig {
    // ... existing variants
    
    /// YourService signer configuration
    YourService {
        /// Environment variable for YourService API key
        api_key_env: String,
        /// Environment variable for YourService API secret
        api_secret_env: String,
        /// Environment variable for YourService wallet ID
        wallet_id_env: String,
    },
}
```

Also update the `build_signer_from_config` method in the same file:

```rust
impl SignerConfig {
    pub async fn build_signer_from_config(config: &SignerConfig) -> Result<KoraSigner, KoraError> {
        match &config.config {
            // ... existing cases
            
            SignerTypeConfig::YourService { api_key_env, api_secret_env, wallet_id_env } => {
                let api_key = get_env_var_for_signer(api_key_env, &config.name)?;
                let api_secret = get_env_var_for_signer(api_secret_env, &config.name)?;
                let wallet_id = get_env_var_for_signer(wallet_id_env, &config.name)?;

                let signer = YourServiceSigner::new(api_key, api_secret, wallet_id)
                    .map_err(|e| {
                        KoraError::ValidationError(format!(
                            "Failed to create YourService signer '{}': {}",
                            config.name, e
                        ))
                    })?;

                Ok(KoraSigner::YourService(signer))
            }
        }
    }
}
```

And update the validation method:

```rust
fn validate_individual_signer_config(&self, index: usize) -> Result<(), KoraError> {
    match &self.config {
        // ... existing cases
        
        SignerTypeConfig::YourService { api_key_env, api_secret_env, wallet_id_env } => {
            let env_vars = [
                ("api_key_env", api_key_env),
                ("api_secret_env", api_secret_env),
                ("wallet_id_env", wallet_id_env),
            ];
            for (field_name, env_var) in env_vars {
                if env_var.is_empty() {
                    return Err(KoraError::ValidationError(format!(
                        "YourService signer '{}' must specify non-empty {}",
                        self.name, field_name
                    )));
                }
            }
        }
    }
    Ok(())
}
```

### Step 6: Add Initialization Logic

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

### Step 7: Export Your Module

Update `crates/lib/src/signer/mod.rs`:

```rust
pub mod your_service;
// ... other modules

pub use your_service::types::YourServiceSigner;
```

## Testing Your Integration

### Environment Variables

Define a `example-signer.toml` with your signer's configuration and necessary environment variables defined. Add the example environment variables to the following files: `.env.example` and `.env` in the root of the project, and in `./sdks/ts/`:

- `YOUR_SERVICE_API_KEY`: The API key for your service.
- `YOUR_SERVICE_API_SECRET`: The API secret for your service.
- `YOUR_SERVICE_WALLET_ID`: The wallet ID for your service.

### Integration Tests

Kora uses a unified test runner (`tests/src/bin/test_runner.rs`) that manages all integration testing phases including TypeScript tests. To add tests for your new signer:

#### 1. Add Test Configuration

Create a new signer configuration file in `tests/src/common/fixtures/` for your service:

```toml
# tests/src/common/fixtures/signers-your-service.toml
[[signers]]
name = "yourservice_main"
type = "your_service"
api_key_env = "YOUR_SERVICE_API_KEY"
api_secret_env = "YOUR_SERVICE_API_SECRET"
wallet_id_env = "YOUR_SERVICE_WALLET_ID"
```

#### 2. Add Test Phase to Test Runner

Update `tests/src/test_runner/test_cases.toml` to include a test phase for your signer:

```toml
[test.your_service]
name = "YourService Signer Tests"
config = "tests/src/common/fixtures/kora-test.toml"
signers = "tests/src/common/fixtures/signers-your-service.toml"
port = "8090"  # Use a unique port
tests = ["your_service"]
```

#### 3. TypeScript SDK Integration

For TypeScript SDK testing with your signer:

1. Update `sdks/ts/test/setup.ts` to recognize your signer type:
   - Add environment variable handling for `KORA_SIGNER_TYPE=your-service`

2. Add a test script in `sdks/ts/package.json`:
   ```json
   "test:integration:your-service": "KORA_SIGNER_TYPE=your-service pnpm test integration.test.ts"
   ```

#### 4. Running Tests

Make sure your environment is set up:

```bash
# Install binaries and dependencies
make install
make install-ts-sdk
make build-ts-sdk

# Set environment variables for your service
export YOUR_SERVICE_API_KEY="your_key"
export YOUR_SERVICE_API_SECRET="your_secret"
export YOUR_SERVICE_WALLET_ID="your_wallet"
```

Run tests using the unified test runner:

```bash
# Run all integration tests (includes your new signer phase)
make test-integration

# Run tests with verbose output
make test-integration-verbose

# Run specific test phase with filter
cargo run -p tests --bin test_runner -- --phases your_service
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

### Configure Signer.toml

\```bash
[[signers]]
name = "yourservice_main"
type = "your_service"
api_key_env = "YOUR_SERVICE_API_KEY"
api_secret_env = "YOUR_SERVICE_API_SECRET"
wallet_id_env = "YOUR_SERVICE_WALLET_ID"
\```


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
