# Public API reference

The public API is exported from `lanqrus`. Modules and protocol internals are not public.

## Handshake functions

### `client_handshake`

```rust
pub async fn client_handshake<S>(
    stream: S,
) -> Result<PendingConnection<S>, LanQRusError>
where
    S: AsyncRead + AsyncWrite + Unpin;
```

Runs the initiator side of `Noise_XX_25519_ChaChaPoly_BLAKE2s`. The complete handshake has a 30-second timeout. It returns a pending connection only after Noise handshake completion.

### `server_handshake`

```rust
pub async fn server_handshake<S>(
    stream: S,
) -> Result<PendingConnection<S>, LanQRusError>
where
    S: AsyncRead + AsyncWrite + Unpin;
```

Runs the responder side with the same protocol and 30-second handshake timeout.

Both functions consume the stream. If they succeed, use `PendingConnection` to approve or wait for approval. Do not use the original stream afterward.

## `PendingConnection<S>`

Generic over a Tokio-compatible asynchronous stream.

### `sas() -> &str`

Returns the eight-digit, zero-padded SAS derived for this session. The value is for display and out-of-band comparison. It is not a persistent identity or an authentication secret.

### `approve(entered_sas: &str) -> Result<SecureStream<S>, LanQRusError>`

Compares `entered_sas` with the locally derived SAS, sends the appropriate approve or reject control frame, waits for the matching acknowledgment, and returns a secure stream on approval.

Semantics:

- A mismatch sends rejection and returns `VerificationFailed`.
- A peer reject acknowledgment after a local mismatch also reports `VerificationFailed`.
- A matching local SAS followed by peer rejection returns `PeerRejected`.
- Invalid or unexpected control frames return `InvalidApproval`.
- The approval exchange has a 30-second timeout and returns `Timeout`.
- This method consumes `self`.

Callers must pass the value observed by the operator as the peer's SAS, not a locally generated value copied automatically.

### `wait_for_approval() -> Result<SecureStream<S>, LanQRusError>`

Waits for the initiating peer's approve or reject control frame. It sends the corresponding acknowledgment and returns `SecureStream` only for approval.

Semantics:

- Approval returns `SecureStream`.
- Peer rejection returns `PeerRejected`.
- Invalid or unexpected control frames return `InvalidApproval`.
- The exchange has a 30-second timeout and returns `Timeout`.
- This method consumes `self`.

## `SecureStream<S>`

Wraps an asynchronous byte stream after successful SAS approval.

### `send(plaintext: &[u8]) -> Result<(), LanQRusError>`

Encrypts one application message and writes it as exactly one frame. Concurrent sends are serialized. A send after closure returns `ConnectionClosed`. The maximum plaintext length is 65,519 bytes (`u16::MAX - 16`) because each frame reserves 16 bytes for the transport authentication tag.

On encryption, I/O, conversion, or cancellation failures that may alter or consume the send path, the stream becomes unusable. `send` must not be retried on the same stream after such a failure.

### `receive() -> Result<Vec<u8>, LanQRusError>`

Waits for and decrypts exactly one complete frame. The result contains the original plaintext and excludes the transport authentication tag.

Concurrent receive calls are serialized, but no ordering guarantee is made about which waiting task receives the next frame. A receive wait is wakeable through `close()`.

If the frame is too short, authentication fails, I/O fails, or cancellation occurs after the frame or transport state may have been consumed, the stream becomes unusable.

### `close() -> Result<(), LanQRusError>`

Marks the connection closed, waits for an in-progress send, and shuts down the underlying write half. It is safe to call more than once and wakes pending receive operations.

## Pairing convenience API

See [pairing.md](pairing.md) for `LanQRusServer`, `LanQRusClient`, and `ConnectionInfo`.

## `LanQRusError`

| Variant | Meaning |
|---|---|
| `InvalidConnectionInfo` | QR payload or connection information is malformed, unsupported, too large, or uses a public/zero address |
| `NoLANAddress` | No private LAN IPv4 interface is available for server binding |
| `Io(std::io::Error)` | Tokio or OS I/O failure |
| `Noise(snow::Error)` | Noise protocol or cryptographic-state failure |
| `ConnectionClosed` | Stream was closed or became unusable |
| `VerificationFailed` | Entered SAS differs from the locally derived SAS |
| `InvalidCode` | Reserved public error variant for invalid verification input |
| `FrameTooLarge` | Handshake or application plaintext/ciphertext frame exceeds supported length |
| `InvalidFrame` | Frame length or payload structure is invalid |
| `Timeout` | Handshake or approval exchange exceeded 30 seconds |
| `InvalidApproval` | Unexpected, malformed, or mismatched approval control frame |
| `PeerRejected` | Peer declined the connection |
| `HandshakeIncomplete` | Noise state did not finish despite successful message exchange |

Callers should handle `PeerRejected`, `VerificationFailed`, `Timeout`, `InvalidApproval`, and I/O/authentication failures without opening a new application-level success path. A failed or cancelled stream is no longer retryable.
