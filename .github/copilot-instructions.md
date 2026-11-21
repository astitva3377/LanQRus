# LanQRus project instructions

## Project overview

- LanQRus is a Rust 2024 library for authenticated, message-oriented encrypted
  communication over arbitrary Tokio `AsyncRead + AsyncWrite` streams.
- The public API is exported from `src/lib.rs`.
- Keep cryptographic behavior, public types, error variants, and wire formats
  backward-compatible unless the request explicitly changes them.
- This is security-sensitive code. Do not weaken verification, nonce handling,
  frame validation, or cancellation behavior.

## Architecture

- `src/handshake.rs`: Noise XX handshake, SAS derivation, approval/rejection,
  and 30-second approval timeout.
- `src/stream.rs`: encrypted message framing and `SecureStream` lifecycle.
- `src/protocol.rs`: control-message magic, version, and control-frame decoder.
- `src/error.rs`: public `LanQRusError` variants.
- `examples/client.rs` and `examples/server.rs`: operator-facing TCP examples.
- `tests/handshake.rs`: end-to-end network, encryption, rejection, concurrency,
  cancellation, and MITM behavior tests.

## Rust conventions

- Preserve the formatting and import style configured by `rustfmt.toml`.
  Run `cargo fmt --all` after source or test changes.
- Use existing `LanQRusError` variants and `thiserror` conversions rather than
  inventing ad hoc error strings for library code.
- Do not add `unsafe` code.
- Keep public APIs and protocol state transitions explicit and documented.
- Preserve generic stream support: handshake and stream types must continue to
  work with Tokio-compatible asynchronous streams.
- Use `Zeroizing` for secret material that is currently derived with it; do not
  log private keys, handshake state, SAS derivation inputs, or plaintext data.

## Protocol and security requirements

- The selected Noise protocol is `Noise_XX_25519_ChaChaPoly_BLAKE2s`.
- The SAS is an eight-digit value derived with the handshake hash and HKDF-SHA256.
  Do not replace or alter this derivation casually.
- Preserve the approval and rejection exchange defined in `src/handshake.rs`
  and `src/protocol.rs`.
- Before changing `MAGIC`, `VERSION`, control opcodes, framing lengths, or
  cipher parameters, update all affected code and tests and call out the wire
  compatibility impact.
- A send or receive operation that may have consumed or modified a frame must
  leave `SecureStream` unusable when it fails or is cancelled.
- Keep concurrent sends serialized and preserve one-message framing semantics.
- Do not convert cancellation, I/O, peer disconnect, or authentication failures
  into success-shaped results.

## Testing

- Add or update focused `#[tokio::test]` cases in `tests/handshake.rs` for
  behavior changes.
- Reuse the existing real-TCP listener and handshake setup patterns. Avoid
  mocking cryptographic state or protocol transitions.
- Run `cargo test --all-targets` after code changes.
- For framing, approval, concurrency, close, or cancellation changes, run
  targeted tests repeatedly when timing-sensitive behavior is involved.
- Run `cargo fmt --all -- --check` and `cargo clippy --all-targets --all-features -- -D warnings`
  before finalizing Rust changes.

## Build and examples

- The Docker targets build the `server` and `client` examples; keep those
  examples compiling and preserve their `SERVER_ADDR` environment variable
  behavior.
- Run `docker compose up --build` only to verify containerized end-to-end
  behavior, and stop it after the check.
