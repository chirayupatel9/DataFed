# DataFed Repository Server (Rust Implementation)

This is a Rust implementation of the DataFed Repository Server that provides the exact same functionality as the C++ version, using ZeroMQ for messaging and Protocol Buffers for serialization.

## Features

- **ZeroMQ Integration**: Full ZeroMQ support with DEALER/ROUTER socket patterns
- **Protocol Buffer Support**: Complete Protocol Buffer message definitions and serialization
- **Proxy Pattern**: ZeroMQ proxy implementation for load balancing across worker threads
- **Security**: Mock credentials system matching the C++ implementation
- **Async Runtime**: Built on Tokio for high-performance async I/O
- **Message Routing**: Protocol Buffer message type-based routing and handling

## Architecture

The implementation follows the exact same architecture as the C++ version:

### Components

1. **RepoServer**: Main server that manages the ZeroMQ proxy and worker threads
2. **RequestWorker**: Worker threads that process incoming messages
3. **ZeroMQCommunicator**: ZeroMQ socket abstraction layer
4. **MessageFactory**: Factory for creating Protocol Buffer messages
5. **Protocol Buffer Definitions**: Complete message schemas matching the C++ version

### Message Flow

1. External clients connect to the proxy server socket (TCP)
2. Proxy forwards messages to worker threads via in-process sockets
3. Workers process messages and send responses back through the proxy
4. Proxy forwards responses back to external clients

## Protocol Buffer Messages

The implementation supports all the same Protocol Buffer messages as the C++ version:

### Anonymous Messages (Protocol ID = 1)
- `VersionRequest` / `VersionReply`

### Authenticated Messages (Protocol ID = 2)
- `RepoDataDeleteRequest`
- `RepoDataGetSizeRequest` / `RepoDataSizeReply`
- `RepoPathCreateRequest`
- `RepoPathDeleteRequest`
- `AckReply` / `NackReply`

## Usage

### Running the Server

```bash
# Basic usage
cargo run

# With custom options
cargo run -- --port 7512 --threads 4 --core-server tcp://localhost:7512

# Available options
cargo run -- --help
```

### Using the Client

```rust
use repo_server_rs::DataFedClient;

// Create a client
let mut client = DataFedClient::new("localhost", 7512)?;

// Check server version
client.check_server_version("tcp://localhost:7512")?;

// Get version info
let version_info = client.get_version()?;
println!("Version: {}", version_info);

// Create a repository path
let response = client.create_path("/test/path".to_string())?;
println!("Path created: {}", response);

// Delete data
let response = client.delete_data(
    vec!["record1".to_string()],
    vec!["/path/to/file1".to_string()],
)?;
println!("Data deleted: {}", response);
```

### Running the Example

```bash
cargo run --example basic_client
```

## Dependencies

- **ZeroMQ**: `zmq = "0.3"` - ZeroMQ messaging library
- **Protocol Buffers**: `prost = "0.12"` - Protocol Buffer serialization
- **Async Runtime**: `tokio = "1"` - Async runtime for high-performance I/O
- **Logging**: `tracing = "0.1"` - Structured logging
- **CLI**: `clap = "4"` - Command-line argument parsing

## Building

### Prerequisites

- Rust 1.70+ with Cargo
- ZeroMQ development libraries
- Protocol Buffer compiler

### Build Commands

```bash
# Build the library
cargo build

# Build with release optimizations
cargo build --release

# Run tests
cargo test

# Run the example
cargo run --example basic_client
```

## Configuration

The server can be configured using a TOML configuration file, with command-line arguments as overrides.

### Configuration File

The server looks for configuration in the following order:
1. `config.toml` in the current directory
2. `/etc/datafed/repo-server.toml`
3. `~/.config/datafed/repo-server.toml`
4. Default values if no config file is found

### Example Configuration

```toml
[server]
port = 10000
num_worker_threads = 4
core_server = "tcp://localhost:9998"
globus_collection_path = "/mnt/datafed-repo"
cred_dir = "/mnt/storage/rust/DataFed/"

[logging]
level = "info"
console = true
file = false

[zmq]
receive_timeout_ms = 5000
poll_timeout_ms = 10
debug = false

[security]
curve_enabled = false
public_key = "your-public-key"
private_key = "your-private-key"

[performance]
max_message_size = 1048576
worker_idle_timeout = 300
connection_pooling = true

[monitoring]
metrics_enabled = false
health_check_enabled = false
health_check_port = 10001
```

### Command-Line Arguments

Command-line arguments override configuration file values:

- `--port`: Server port (overrides config)
- `--threads`: Number of worker threads (overrides config)
- `--core-server`: Core server address (overrides config)
- `--globus-path`: Globus collection path (overrides config)
- `--cred-dir`: Credentials directory (overrides config)
- `--log-level`: Logging level (overrides config)

### Development Configuration

For development, you can copy `config-dev.toml` to `config.toml` to get development-friendly settings:

```bash
cp config-dev.toml config.toml
```

The development configuration includes:
- Debug logging enabled
- Shorter timeouts for faster iteration
- Development-friendly paths (`/tmp/...`)
- Health check endpoint enabled

## Mock Credentials

For testing purposes, the implementation uses the same mock credentials as the C++ version:

- **Public Key**: `4X1hKiU5pdwdk.s7&=Q2(b1]p!^Nj=Dnk2&7vh@f`
- **Private Key**: `G1DpacgVoCcRmLYQ6PA8:Q$]/w5SE*Qm?)}L!@Gv`

## Message Types

The implementation uses the following message type IDs:

- `1`: Version request/reply (anonymous)
- `2`: Version reply (anonymous)
- `3`: Repo data delete request (authenticated)
- `4`: Repo data get size request (authenticated)
- `5`: Repo path create request (authenticated)
- `6`: Repo path delete request (authenticated)
- `7`: Ack reply (authenticated)
- `8`: Nack reply (authenticated)
- `9`: Repo data size reply (authenticated)

## Error Handling

The implementation provides comprehensive error handling:

- **Network Errors**: ZeroMQ connection and communication errors
- **Protocol Errors**: Protocol Buffer serialization/deserialization errors
- **Timeout Errors**: Message receive/send timeouts
- **Validation Errors**: Invalid message types or payloads

## Logging

The implementation uses structured logging with the `tracing` crate:

```rust
use tracing::{info, error, debug, warn};

info!("Server started on port {}", port);
error!("Failed to process message: {}", error);
debug!("Received message: {:?}", message);
warn!("Version check failed, continuing anyway");
```

## Testing

```bash
# Run all tests
cargo test

# Run tests with output
cargo test -- --nocapture

# Run specific test
cargo test test_message_factory
```

## Performance

The Rust implementation provides:

- **High Performance**: Zero-copy Protocol Buffer serialization
- **Low Latency**: Async I/O with Tokio runtime
- **Scalability**: Multi-threaded worker pool
- **Memory Safety**: Rust's ownership and borrowing system

## Compatibility

This implementation is designed to be fully compatible with the C++ DataFed server:

- Same Protocol Buffer message definitions
- Same ZeroMQ socket patterns
- Same message routing logic
- Same error handling behavior
- Same mock credentials for testing

## Development

### Project Structure

```
src/
├── lib.rs              # Main library entry point
├── main.rs             # Server binary
├── proto/              # Protocol Buffer definitions
├── zmq_communicator.rs # ZeroMQ implementation
├── request_worker.rs   # Worker thread implementation
└── server.rs           # Server implementation
proto/                  # Protocol Buffer .proto files
examples/               # Example usage
```

### Adding New Message Types

1. Add the message definition to the appropriate `.proto` file
2. Regenerate the Protocol Buffer code: `cargo build`
3. Add a message handler in `RequestWorker::setup_message_handlers()`
4. Implement the handler function
5. Add client methods in `DataFedClient` if needed

## License

Apache-2.0
