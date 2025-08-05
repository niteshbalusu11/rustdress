# First stage: Build the Rust application
FROM rust:1.82 AS builder
# Install system dependencies
RUN apt-get update && \
    apt-get install -y cmake pkg-config && \
    apt-get clean && \
    rm -rf /var/lib/apt/lists/*
# Set up the working directory
WORKDIR /app
# Copy the Rust project's source files
COPY . .
# Build the Rust project
RUN cargo build --release

# Second stage: Create a smaller runtime image
FROM ubuntu:22.04
# Install necessary runtime dependencies
RUN apt-get update && \
    apt-get install -y libssl3 ca-certificates curl && \
    apt-get clean && \
    rm -rf /var/lib/apt/lists/*
# Copy the compiled binary from the builder stage
COPY --from=builder /app/target/release/rustdress /usr/local/bin/
# Run the Rust binary
CMD ["rustdress", "--config", "/app/config.toml"]
