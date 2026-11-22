# Pairing API

LanQRus provides a TCP convenience layer in addition to generic handshake functions.

## `LanQRusServer`

### `bind() -> Result<LanQRusServer, LanQRusError>`

Discovers the first private LAN IPv4 interface, binds a TCP listener on `0.0.0.0:0`, and constructs connection information containing that interface address and the assigned ephemeral port.

Errors:

- `NoLANAddress` when no private IPv4 interface is found
- `Io` for interface or listener failures
- `InvalidConnectionInfo` if address construction fails validation

### `connection_info() -> &ConnectionInfo`

Returns the address and port that a client should use.

### `accept(self) -> Result<PendingConnection<TcpStream>, LanQRusError>`

Accepts exactly one TCP connection and runs `server_handshake`. It consumes the server wrapper. The returned pending connection is not yet approved.

## `LanQRusClient`

### `connect(info: &ConnectionInfo) -> Result<PendingConnection<TcpStream>, LanQRusError>`

Validates the supplied connection information by reference, opens a TCP connection to `info.address()`, and runs `client_handshake`.

## `ConnectionInfo`

`ConnectionInfo` is a serializable, private-LAN-only description of a LanQRus TCP server.

### Address

`address() -> &SocketAddrV4` returns the private IPv4 address and TCP port.

### Format version

The serialized QR format version is `1`. Unknown fields are rejected.

The JSON shape is:

```json
{"version":1,"address":"192.168.1.20:65432"}
```

The exact JSON field names and format version are compatibility-sensitive.

### `to_qr_payload()`

```rust
pub fn to_qr_payload(&self) -> Result<String, LanQRusError>
```

Serializes connection information as JSON. The payload must remain at or below 2,048 bytes.

### `from_qr_payload(payload: &str)`

```rust
pub fn from_qr_payload(payload: &str) -> Result<Self, LanQRusError>
```

Deserializes and validates JSON. The string must be non-empty and no longer than 2,048 bytes. Unknown fields, unsupported versions, public addresses, unspecified addresses, and zero ports are rejected with `InvalidConnectionInfo`.

The `SocketAddrV4` uses Rust's standard string representation. Applications may generate QR codes or otherwise transport the exact returned string.

## Pairing sequence

1. Server binds and exposes `ConnectionInfo`.
2. Client parses `ConnectionInfo` and connects.
3. Both endpoints run the Noise XX handshake.
4. Both endpoints display the derived SAS.
5. An operator compares the two SAS values through a trusted channel.
6. The client calls `approve(peer_sas)`.
7. The server calls `wait_for_approval()`.
8. Both endpoints receive `SecureStream` only after approval acknowledgment succeeds.
