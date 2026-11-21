# Protocol and framing

## Handshake

LanQRus uses:

```text
Noise_XX_25519_ChaChaPoly_BLAKE2s
```

The handshake is asymmetric: the initiating peer sends the first message, and
the responding peer sends the second. After the final handshake message, both
peers derive their own SAS from the handshake hash and raw split keys.

The in-memory SAS is eight decimal digits. It is not a persistent identity
credential; applications must compare it out of band for every session.

## Handshake approval

After the Noise handshake, application approval uses a four-byte control
frame:

```text
magic "LQ" | version 1 | operation
```

Operations are:

- `1`: approve
- `2`: reject
- `3`: reject acknowledgment

An approval waits up to 30 seconds for the peer decision. Any mismatch,
invalid control frame, timeout, or peer rejection prevents the secure stream
from being returned.

## Application framing

Application frames use an unsigned 16-bit big-endian body length. The
plaintext limit is `u16::MAX - 16` bytes because 16 bytes are reserved for the
Noise transport authentication tag.

Every `send` call encrypts and emits one frame. Every `receive` call reads and
decrypts exactly one frame.

## Compatibility

Changing the Noise pattern, SAS derivation, control-frame format, message
length, or cipher parameters can make peers incompatible. Update examples and
tests together and document the wire-format impact.
