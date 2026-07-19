FROM rust:1.97.1-slim AS build
COPY . /app
WORKDIR /app
RUN cargo build --release

FROM debian:stable-20260713-slim
WORKDIR /app
COPY --from=build /app/target/release/relay-rank .
