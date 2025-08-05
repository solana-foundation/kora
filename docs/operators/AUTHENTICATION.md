# Kora Authentication

Kora supports two optional authentication methods for securing your RPC endpoint: API Key and HMAC authentication. This guide covers setup, implementation, and security best practices.

Authentication is optional but **strongly recommended** for production deployments. Without authentication, anyone who discovers your Kora endpoint can submit transactions and consume your SOL balance.

| Method | Security Level | Use Case | Complexity |
|--------|---------------|----------|------------|
| **None** | ⚠️ None | Development, testing, high-margin pricing | None |
| **API Key** | Basic | Internal apps, trusted clients | Low |
| **HMAC** | High | Public APIs, untrusted networks | Medium |
| **Both** | Maximum | High-security environments | Medium |

In this document:

- [API Key Authentication](#api-key-authentication)
- [HMAC Authentication](#hmac-authentication)
- [Combined Authentication](#combined-authentication)
- [Security Best Practices](#security-best-practices)
- [Exempt Endpoints](#exempt-endpoints)
- [Troubleshooting](#troubleshooting)

## API Key Authentication

Simple shared secret authentication using HTTP headers. You can generate a new API key using the `openssl` command (or a similar command) in your terminal:

```bash
openssl rand -hex 32
```

### Server Configuration


- Add a `KORA_API_KEY` to your .env (environment variables) (has priority)
  or
- Add an `api_key` to your `kora.toml`:

```toml
[kora]
rate_limit = 100
api_key = "kora_live_sk_1234567890abcdef"  # Use a strong, unique key
```

This key will be globally required for all requests to the Kora RPC endpoint.

### Client Implementation

Include the API key in the `x-api-key` header with every request:

**cURL Example:**
```bash
curl -X POST http://localhost:8080 \
  -H "Content-Type: application/json" \
  -H "x-api-key: kora_live_sk_1234567890abcdef" \
  -d '{"jsonrpc": "2.0", "method": "getConfig", "id": 1}'
```

**JavaScript Example:**
<!-- TODO: Update after we have TS SDK with headers -->
```javascript
async function callKora(method, params = []) {
  const response = await fetch('http://localhost:8080', {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
      'x-api-key': process.env.KORA_API_KEY //'kora_live_sk_1234567890abcdef'
    },
    body: JSON.stringify({
      jsonrpc: '2.0',
      method,
      params,
      id: 1
    })
  });
  
  return response.json();
}

const config = await callKora('getConfig');
console.log(config);
```

## HMAC Authentication

Instead of sending an API key with every request, HMAC creates a unique cryptographic signature that proves you know a secret without revealing it. Each signature includes a timestamp and expires after 5 minutes, so intercepted requests can't be replayed. Attackers can't create new requests because they don't have your secret key.

### Server Configuration

- Add `KORA_HMAC_SECRET` to your .env (environment variables) (has priority)
  or
- Add a global `hmac_secret` to your `kora.toml` (minimum 32 characters--you can generate one with `openssl rand -hex 32` or similar):

```toml
[kora]
rate_limit = 100
hmac_secret = "kora_hmac_your-strong-hmac-secret-key"
```

### How HMAC Works

1. Client creates a message by concatenating: `{timestamp}{request_body}`
2. Client signs the message using HMAC-SHA256 with the shared secret
3. Client sends the request with timestamp and signature headers
4. Server validates the signature and timestamp (must be within 5 minutes)

### Client Implementation

To use HMAC client-side, you can use the `crypto` library in JavaScript:
1. Create a timestamp
2. Create the request body
3. Create a message by concatenating the timestamp and body (e.g., `message = timestamp + body`)
4. Create a signature by signing the message with the HMAC secret (using the `crypto.createHmac` method)
5. Send the request with the timestamp (`x-timestamp`) and signature (`x-hmac-signature`) headers

**JavaScript Example:**
<!-- TODO: Update after we have TS SDK helper -->
```javascript
const crypto = require('crypto');

async function callKoraHMAC(method, params = []) {
  const timestamp = Math.floor(Date.now() / 1000).toString();
  const body = JSON.stringify({
    jsonrpc: '2.0',
    method,
    params,
    id: 1
  });
  
  // Create HMAC signature
  const message = timestamp + body;
  const signature = crypto
    .createHmac('sha256', process.env.KORA_HMAC_SECRET) // kora_hmac_your-strong-hmac-secret-key
    .update(message)
    .digest('hex');
  
  const response = await fetch('http://localhost:8080', {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
      'x-timestamp': timestamp,
      'x-hmac-signature': signature
    },
    body: body
  });
  
  return response.json();
}

const config = await callKoraHMAC('getConfig');
console.log(config);
```


## Combined Authentication

You can enable both authentication methods simultaneously for maximum security:

```toml
[kora]
rate_limit = 100
api_key = "kora_live_sk_1234567890abcdef"
hmac_secret = "kora_hmac_your-strong-hmac-secret-key"
```

When both are configured, clients must send the `x-api-key`, `x-timestamp`, and `x-hmac-signature` headers.

## Security Best Practices

- **Use strong, random keys**: Minimum 32 characters with high entropy
- **Rotate regularly**: Change keys periodically (monthly/quarterly)
- **Secure storage**: Use environment variables or secrets management (Railway secrets, AWS Secrets Manager, etc.)
- **Never hardcode**: Keep keys out of source code and logs
- **Use HTTPS**: Always use TLS in production to protect keys in transit
- **Monitor access**: Watch for unusual authentication patterns or repeated failures

## Exempt Endpoints

The `/liveness` endpoint is always exempt from authentication to allow health checks:

```bash
# This works even with authentication enabled
curl http://localhost:8080/liveness
```

## Troubleshooting

**401 Unauthorized with API Key:**
- Verify the API key is correct and matches server configuration
- Check that the `x-api-key` header is being sent
- Ensure no extra whitespace in the key

**401 Unauthorized with HMAC:**
- Verify timestamp is current (within 5 minutes)
- Check that message construction matches: `{timestamp}{body}`
- Ensure HMAC secret matches server configuration
- Verify signature is lowercase hex
