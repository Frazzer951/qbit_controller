FROM rust:1.84-alpine AS builder

RUN apk add --no-cache \
    musl-dev \
    openssl-dev \
    pkgconf \
    openssl-libs-static \
    gcc

WORKDIR /usr/src/myapp

COPY Cargo.toml ./

# Create a dummy main.rs to build dependencies
RUN mkdir src && \
    echo "fn main() {}" > src/main.rs && \
    cargo build --release && \
    rm -rf src

COPY src ./src
COPY log_config.yml ./
COPY config/example_config.yml ./config/example_config.yml

RUN touch src/main.rs && cargo build --release

FROM alpine:3.21

WORKDIR /
COPY --from=builder /usr/src/myapp/target/release/qbit_controller /qbit_controller
COPY --from=builder /usr/src/myapp/log_config.yml /log_config.yml
COPY run.sh /run.sh

RUN chmod +x /run.sh

ENTRYPOINT ["./run.sh"]
