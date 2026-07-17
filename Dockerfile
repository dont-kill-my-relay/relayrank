FROM rust:1.97.1-slim AS build
RUN mkdir -p /app
COPY . /app
WORKDIR /app
RUN cargo build --release

FROM debian:stable-20260713-slim
RUN mkdir -p /app
WORKDIR /app
COPY --from=build /app/target/release/relay-rank .
