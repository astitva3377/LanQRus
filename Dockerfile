FROM rust:1.99.1-bookworm AS builder

WORKDIR /lanqrus

COPY Cargo.toml Cargo.lock ./
COPY src ./src
COPY examples ./examples

RUN rustc --version

RUN cargo build --release --example server

RUN cargo build --release --example client

FROM debian:bookworm-slim AS server

WORKDIR /lanqrus

COPY --from=builder /lanqrus/target/release/examples/server ./server

CMD ["./server"]

FROM debian:bookworm-slim AS client

WORKDIR /lanqrus

COPY --from=builder /lanqrus/target/release/examples/client ./client

CMD ["./client"]