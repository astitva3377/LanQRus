# LanQRus

LanQRus is a Rust library for authenticated, message-oriented encrypted communication over arbitrary Tokio `AsyncRead + AsyncWrite` streams.
It uses Noise XX and an operator-verifiable eight-digit SAS to prevent unconfirmed connections from becoming trusted.

## Setup

### Requirements

- Rust edition 2024
- Tokio with `rt-multi-thread`, `macros`, `io-util`, `sync`, `time`, and `net` features

Add LanQRus to a Rust 2024 application:

```toml
[dependencies]
lanqrus = "0.1"
tokio = { version = "1", features = ["rt-multi-thread", "macros", "net", "io-util", "sync", "time"] }
```

Clone and build the repository:

```powershell
cargo build --all-targets
```

## Quickstart

The repository includes matching TCP examples. Start the server, then run the client in a second terminal:

```powershell
cargo run --example server
```

```powershell
cargo run --example client
```

On the server:

1. Copy the JSON connection string printed after `QR code`.
2. Start the client and paste that string when prompted.
3. The server prints the eight-digit SAS after the handshake.
4. Compare that SAS with the value shown at the peer through a separate trusted channel.
5. Enter the peer SAS at the client prompt.

The server waits for the client's approval decision. The client only establishes the `SecureStream` after matching values and receiving the approval acknowledgment.

The examples demonstrate the public pairing API. `LanQRusServer::bind()` selects a private LAN IPv4 address and an ephemeral port, and `LanQRusClient::connect()` returns a pending connection.

## Library guide

### Pairing API

```rust
use lanqrus::{LanQRusClient, LanQRusServer};

# async fn example() -> Result<(), lanqrus::LanQRusError> {
let server = LanQRusServer::bind().await?;
let info = server.connection_info().clone();
let (client_pending, server_pending) = tokio::join!(
    LanQRusClient::connect(&info),
    server.accept(),
);
let client_pending = client_pending?;
let server_pending = server_pending?;

let sas = client_pending.sas();
assert_eq!(sas, server_pending.sas());

let (client, server) = tokio::join!(
    client_pending.approve(sas),
    server_pending.wait_for_approval(),
);
let client = client?;
let server = server?;
# let _ = (client, server);
# Ok(())
# }
```

`LanQRusServer::bind()` and `LanQRusServer::accept()` consume one TCP listener and accept one connection. `LanQRusClient::connect()` returns a `PendingConnection<TcpStream>` after the Noise handshake.

For a custom Tokio stream, use `server_handshake(stream)` or `client_handshake(stream)`.

### Secure messages

```rust
secure.send(b"hello").await?;
let plaintext = secure.receive().await?;
secure.close().await?;
```

Each successful `send` emits one encrypted, authenticated frame. Each successful `receive` returns exactly one plaintext message. Concurrent sends are serialized; concurrent receive calls are serialized. After a fatal failure or cancellation that may affect a frame, the stream is unusable.

### Failure handling

Check `LanQRusError` for handshake, approval, framing, I/O, timeout, and lifecycle failures. I/O failures are available as `LanQRusError::Io`, and peer rejection is `LanQRusError::PeerRejected`. Never treat a matching code or SAS display alone as authentication: require an out-of-band comparison.

## Documentation

- [Getting started](docs/getting-started.md)
- [Public API reference](docs/api.md)
- [Pairing API and connection-info format](docs/pairing.md)
- [Protocol and framing specification](docs/protocol.md)
- [Security guide](docs/security.md)
- [Wire and lifecycle specification](docs/specification.md)

## Development

```powershell
cargo fmt --all
cargo test --all-targets
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
```

The Docker targets build the server and client examples. Containerized verification:

```powershell
docker compose up --build
docker compose down
```
