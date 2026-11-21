# LanQRus

LanQRus is a Rust library for authenticated, encrypted, message-oriented
communication over Tokio-compatible asynchronous byte streams.

It uses the `Noise_XX_25519_ChaChaPoly_BLAKE2s` protocol. After the Noise
handshake, both peers receive an eight-digit short authentication string
(SAS). Applications compare those values through an out-of-band channel before
approving the connection.

## Features

- Generic `AsyncRead + AsyncWrite + Unpin` stream support
- Noise XX mutual authentication and forward-secret transport encryption
- Eight-digit SAS verification
- Explicit approve/reject peer exchange
- Framed, authenticated application messages
- Concurrent send serialization and receive cancellation protection

## Requirements

- Rust edition 2024
- Tokio with the features declared in `Cargo.toml`

## Build and test

```powershell
cargo fmt --all
cargo test --all-targets
cargo clippy --all-targets --all-features -- -D warnings
```

## TCP examples

Start the server:

```powershell
cargo run --example server
```

Start the client in another terminal:

```powershell
cargo run --example client
```

Both examples use `127.0.0.1:9000` by default. Set `SERVER_ADDR` to override
it. Compare the SAS printed by each process, enter the peer's value at the
prompt, and only then approve the connection.

## Docker Compose

```powershell
docker compose up --build
docker compose down
```

The compose stack builds separate server and client containers and uses
`server:9000` for the client connection.

## API overview

- `client_handshake(stream)` starts the initiator side.
- `server_handshake(stream)` starts the responder side.
- `PendingConnection::sas()` returns the locally derived verification code.
- `PendingConnection::approve(peer_sas)` approves a matching peer.
- `PendingConnection::reject()` explicitly rejects the peer.
- `SecureStream::send(bytes)` sends one encrypted frame.
- `SecureStream::receive()` receives one complete plaintext frame.
- `SecureStream::close()` closes the stream and wakes pending receivers.

See [the protocol notes](docs/protocol.md) and
[security notes](docs/security.md) for lifecycle and compatibility details.
