# Use the official Rust nightly image as a builder (required for edition 2024)
FROM rustlang/rust:nightly-slim AS builder

# Install build dependencies including C++ compiler
RUN apt-get update && apt-get install -y \
    build-essential \
    && rm -rf /var/lib/apt/lists/*

# Set working directory
WORKDIR /usr/src/app

# Copy the Cargo files
COPY Cargo.toml Cargo.lock ./

# Create a dummy main.rs to build dependencies
RUN mkdir src && echo "fn main() {}" > src/main.rs

# Build dependencies
RUN cargo build --release

# Remove the dummy main.rs and copy the real source code
RUN rm src/main.rs
COPY src/ ./src/

# Build the application
RUN cargo build --release

# Runtime stage
FROM ubuntu:22.04
# debian:bookworn
# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Create a non-root user
RUN useradd --create-home --shell /bin/bash datafed

# Set working directory
WORKDIR /app

# Copy the binary from builder stage
COPY --from=builder /usr/src/app/target/release/repo-server-rs /app/

# Copy configuration files
COPY repo-server.toml ./

# Create necessary directories
RUN mkdir -p logs data credentials

# Change ownership to datafed user
RUN chown -R datafed:datafed /app

# Switch to non-root user
USER datafed

# Expose port (you may need to adjust this based on your configuration)
EXPOSE 10000

# Run the application
CMD ["./repo-server-rs"]
