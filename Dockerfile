FROM rust:1.97.1-alpine3.24 AS builder
RUN rustup target add x86_64-unknown-linux-musl
WORKDIR /app
COPY . /app
RUN cargo build --target x86_64-unknown-linux-musl --release

FROM scratch AS runtime
COPY --from=builder /app/target/x86_64-unknown-linux-musl/release/relay-rank .
