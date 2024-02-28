# First stage: Build the Rust application
FROM rust:1.72.1 AS builder

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
FROM debian:buster-slim

# Install necessary runtime dependencies
RUN apt-get update && \
    apt-get install -y libssl-dev ca-certificates && \
    apt-get clean && \
    rm -rf /var/lib/apt/lists/*

# Copy the compiled binary from the builder stage
COPY --from=builder /app/target/release/rustdress /usr/local/bin/

# Run the Rust binary
CMD ["rustdress"]
