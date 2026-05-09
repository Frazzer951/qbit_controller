FROM rust:1.95-alpine AS builder

RUN apk add --no-cache musl-dev

WORKDIR /usr/src/qbit_controller

COPY Cargo.toml Cargo.lock ./
COPY rust-toolchain.toml ./

RUN mkdir src && \
    echo "fn main() {}" > src/main.rs && \
    cargo build --release --locked && \
    rm -rf src

COPY config ./config
COPY src ./src

RUN touch src/main.rs && cargo build --release --locked

FROM alpine:3.23

RUN apk add --no-cache ca-certificates && \
    addgroup -g 1000 -S qbit && \
    adduser -u 1000 -S -D -H -G qbit qbit && \
    mkdir -p /config && \
    chown -R qbit:qbit /config

WORKDIR /

COPY --from=builder /usr/src/qbit_controller/target/release/qbit_controller /qbit_controller
COPY log_config.yml /log_config.yml
COPY run.sh /run.sh

RUN chmod +x /run.sh

USER qbit

ENTRYPOINT ["./run.sh"]
