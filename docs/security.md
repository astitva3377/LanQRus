# Security guidance

LanQRus is designed for operator-verifiable local sessions. It does not derive
trust solely from an IP address or a self-generated ephemeral key.

## SAS verification

Always display the SAS on both endpoints and require the operator to compare
the values through a separate trusted channel. Calling `approve` with the
peer's displayed value establishes that the peer claimed to match only after
the application has performed the comparison.

Do not automate SAS approval from unverified network metadata.

## Failure behavior

- Authentication failures, I/O failures, cancellations, and protocol violations
  must leave or mark the connection unusable.
- The transport closes after send or receive operations that fail at a point
  where a frame may have been consumed or modified.
- Invalid approval frames and mismatched fingerprints must not establish a
  secure stream.
- Do not add an automatic approval shortcut based on network address,
  connection-info JSON, or SAS equality without operator review.

## Secret handling

Do not log private keys, handshake state, split keys, ciphertext, plaintext,
SAS derivation material, or transport state. Keep derived secret material
zeroized where this codebase already uses `Zeroizing`.

## Development requirements

- Do not add `unsafe` Rust.
- Do not mock cryptographic state in protocol tests.
- Test real TCP handshakes and approval flows.
- Preserve cancellation guards and stream close state in send/receive changes.
- Run the complete test suite and Clippy before changing cryptographic or
  framing behavior.
