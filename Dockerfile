FROM rust:1.97.1-slim AS builder

ARG TARGETARCH

RUN case "${TARGETARCH}" in \
      amd64)  RUST_TARGET=x86_64-unknown-linux-musl ;; \
      arm64)  RUST_TARGET=aarch64-unknown-linux-musl ;; \
      riscv64) RUST_TARGET=riscv64gc-unknown-linux-musl ;; \
      ppc64)  RUST_TARGET=powerpc64-unknown-linux-musl ;; \
      ppc64le) RUST_TARGET=powerpc64le-unknown-linux-musl ;; \
      long64) RUST_TARGET= loongarch64-unknown-linux-musl ;; \
      arm/v7) RUST_TARGET=armv7-unknown-linux-musleabi ;; \
    *)      echo "Unsupported architecture: ${TARGETARCH}" && exit 1 ;; \
    esac && \
    rustup target add ${RUST_TARGET} && \
    echo ${RUST_TARGET} > /tmp/rust-target

WORKDIR /app
COPY . /app
RUN RUST_TARGET=$(cat /tmp/rust-target) && \
    cargo build --release --target ${RUST_TARGET} && \
    cp "target/$RUST_TARGET/release/relay-rank" .

FROM scratch AS runtime

COPY --from=builder /app/relay-rank .
ENTRYPOINT [ "/relay-rank" ]
