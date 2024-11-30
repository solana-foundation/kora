# Kora

A paymaster node that provides a JSON-RPC interface.

## Features

- JSON-RPC server with middleware support
- Health check endpoint (`/liveness`)
- Configurable logging (JSON or standard format)
- Optional metrics endpoint
- Customizable RPC endpoint

## Usage

```bash
cargo run -- [OPTIONS]
```

### Command Line Arguments

```
Options:
  -p, --port <PORT>
          Port number for the RPC server [default: 8080]

  -r, --rpc-url <RPC_URL>
          RPC URL to connect to [default: "http://127.0.0.1:8899"]

  -l, --logging-format <FORMAT>
          Logging format (standard or json) [default: "standard"]

  --metrics-endpoint <ENDPOINT>
          Optional metrics endpoint URL

  -h, --help
          Print help information
```

### Environment Variables

- `RUST_LOG`: Controls log level and filtering (e.g., "info,sqlx=error,sea_orm_migration=error,jsonrpsee_server=warn")

## Development

### Building

```bash
cargo build
```

### Running Tests

```bash
cargo test
```

### Running the Server

1. Start the server with default settings:
   ```bash
   cargo run
   ```

2. Start with custom port and RPC URL:
   ```bash
   cargo run -- --port 9000 --rpc-url http://localhost:8899
   ```

3. Enable JSON logging:
   ```bash
   cargo run -- --logging-format json
   ```

4. Enable metrics:
   ```bash
   cargo run -- --metrics-endpoint http://localhost:9090
   ```
