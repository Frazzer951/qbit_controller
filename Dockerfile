FROM rust:latest as builder

# Set the working directory
WORKDIR /usr/src/myapp

# Copy the Rust project files
COPY . .

# Build your project
RUN cargo build --release

# Final stage
FROM ubuntu:latest

# Update and install necessary libraries
RUN apt-get update && \
    apt-get install -y libssl-dev && \
    rm -rf /var/lib/apt/lists/*

WORKDIR /
COPY --from=builder /usr/src/myapp/target/release/qbit_controller /qbit_controller
COPY --from=builder /usr/src/myapp/log_config.yml /log_config.yml
COPY run.sh /run.sh

RUN chmod +x /run.sh

ENTRYPOINT ["/run.sh"]
