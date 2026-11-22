# Getting started

## Add the dependency

LanQRus is a Rust 2024 library and uses Tokio for asynchronous I/O.

```toml
[dependencies]
lanqrus = "0.1"
tokio = { version = "1", features = ["rt-multi-thread", "macros", "net", "io-util", "sync", "time"] }
```

## Run the supplied examples

In one terminal:

```powershell
cargo run --example server
```

In another terminal:

```powershell
cargo run --example client
```

The server prints a JSON connection string and listens on a private LAN IPv4 address. Paste that string into the client. Each side derives the same eight-digit SAS. Compare the values through a channel independent of the network connection, then enter the server's SAS in the client.

## Connect with a custom stream

The handshake functions accept any Tokio-compatible stream implementing `AsyncRead + AsyncWrite + Unpin`.

```rust
use lanqrus::{LanQRusError, client_handshake};
use tokio::net::TcpStream;

async fn connect(address: &str) -> Result<(), LanQRusError> {
    let stream = TcpStream::connect(address).await?;
    let pending = client_handshake(stream).await?;
    println!("SAS: {}", pending.sas());

    let secure = pending.approve(pending.sas()).await?;
    secure.send(b"hello").await?;
    let response = secure.receive().await?;
    println!("{}", String::from_utf8_lossy(&response));
    secure.close().await?;
    Ok(())
}
```

Use this pattern only when the application performs a real out-of-band SAS comparison. Supplying `pending.sas()` automatically is not verification.

## Server side

```rust
use lanqrus::LanQRusServer;

async fn run() -> Result<(), lanqrus::LanQRusError> {
    let server = LanQRusServer::bind().await?;
    println!("{}", server.connection_info().to_qr_payload()?);
    let pending = server.accept().await?;
    println!("SAS: {}", pending.sas());
    let secure = pending.wait_for_approval().await?;
    secure.send(b"ready").await?;
    Ok(())
}
```

`LanQRusServer::bind()` discovers a private LAN IPv4 address and binds port `0`, so it is suitable for LAN pairing workflows. `accept()` completes the responder-side Noise handshake and returns a `PendingConnection`.

## Next steps

- Read the [public API reference](api.md).
- Read [pairing and connection-info details](pairing.md).
- Review [security requirements](security.md) before integrating approval.
