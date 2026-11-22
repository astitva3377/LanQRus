# LanQRus wire and lifecycle specification

This specification describes the observable protocol behavior. It is intended for implementation and compatibility review.

## Cryptographic handshake

- Noise pattern: `Noise_XX_25519_ChaChaPoly_BLAKE2s`
- Handshake initiator: client
- Handshake responder: server
- Ephemeral key exchange: X25519
- AEAD: ChaCha20-Poly1305
- Hash: BLAKE2s
- Handshake timeout: 30 seconds

The final handshake message is followed by transport split. The raw split keys and handshake hash are used to derive the SAS.

## SAS derivation

LanQRus derives SAS material from:

- The Noise handshake hash
- Both raw split keys
- HKDF-SHA256
- Info string `lanqrus SAS v1`

The HKDF output is eight bytes interpreted as a big-endian `u64`, reduced modulo 100,000,000, and formatted as exactly eight decimal digits with leading zeroes.

The SAS is ephemeral and is not a persistent identity. It must be compared through an independent trusted channel.

## Handshake frames

Handshake messages use a two-byte, unsigned, big-endian payload length followed by the payload:

```text
length: 2 bytes
payload: length bytes
```

A zero-length handshake frame is invalid.

## Approval frames

All approval control frames are exactly four bytes:

```text
0..1  "LQ"
2     version = 1
3     opcode
```

| Opcode | Name | Meaning |
|---:|---|---|
| 1 | Approve | Initiator accepts the matching operator SAS |
| 2 | Reject | Initiator's entered or operator-verified SAS does not match |
| 3 | RejectAck | Responder acknowledges rejection |
| 4 | ApproveAck | Responder acknowledges approval |

Unknown magic, version, opcode, or lengths invalidate approval.

The approval and acknowledgment exchange also has a 30-second timeout.

## Application frames

Application frames use:

```text
length: 2 bytes, unsigned big-endian
ciphertext: length bytes
```

`length` is the ciphertext length, not the plaintext length. The ciphertext must contain at least the 16-byte Noise authentication tag.

Maximum plaintext: `65,519` bytes.
Maximum ciphertext: `65,535` bytes.

Each `send` call must produce one frame. Each `receive` call must consume one complete frame and return one plaintext message.

Noise transport nonces must advance once per successfully encrypted or decrypted transport message. LanQRus serializes send operations and receive operations.

## Failure and close semantics

An operation may fail because of:

- Closed lifecycle state
- I/O failure
- Invalid framing
- Noise authentication or cryptographic failure
- Cancellation
- Approval or handshake timeout
- Peer rejection

Once a send or receive operation may have consumed or modified a frame, failure or cancellation marks `SecureStream` unusable. The stream must remain unusable after a fatal receive or send failure.

`close()` is idempotent in application behavior, wakes pending receive operations, waits for an in-progress send, and shuts down the underlying write half.

## Compatibility rules

The following are wire-compatibility-sensitive:

- Noise pattern
- SAS HKDF input and formatting
- Handshake frame format
- Approval magic and version
- Approval opcode values
- Approval timeout
- Application frame length format
- Authentication-tag size
- Cipher and hash parameters

Changes require coordinated implementation updates, tests, and documentation, and should be identified as incompatible unless both peers support a negotiated or versioned transition.
