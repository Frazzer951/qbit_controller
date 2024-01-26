FROM rust:latest as builder

# Set the working directory
WORKDIR /usr/src/myapp

# Copy the Rust project files
COPY . .

# Build your project
RUN cargo build --release

# Copy the required files
COPY log_config.yml .

# Final stage
FROM ubuntu:latest

# Update and install necessary libraries
RUN apt-get update && \
    apt-get install -y libssl-dev && \
    rm -rf /var/lib/apt/lists/*

# Copy the build artifact from the build stage
COPY --from=builder /usr/src/myapp/target/release/qbit_controller .

# Copy the required files
COPY --from=builder /usr/src/myapp/log_config.yml .

# Set the startup command to run your binary
CMD ["./qbit_controller"]
