# Stage 1: Chef (Prepare)
FROM lukemathwalker/cargo-chef:latest-rust-1.91.0 AS chef
WORKDIR /app

# Install protobuf compiler and dev files (includes google well-known types)
RUN apt-get update && apt-get install -y --no-install-recommends \
    protobuf-compiler \
    libprotobuf-dev \
    && rm -rf /var/lib/apt/lists/*

# Stage 2: Planner (Compute recipe)
FROM chef AS planner
COPY Cargo.toml Cargo.lock ./
COPY auth-service ./auth-service
COPY billing-service ./billing-service
COPY document-service ./document-service
COPY genai-service ./genai-service
COPY invoicing-service ./invoicing-service
COPY ledger-service ./ledger-service
COPY notification-service ./notification-service
COPY payment-service ./payment-service
COPY reconciliation-service ./reconciliation-service
COPY service-core ./service-core
COPY proto ./proto
RUN cargo chef prepare --recipe-path recipe.json --bin notification-service

# Stage 3: Builder (Build dependencies and binary)
FROM chef AS builder
COPY --from=planner /app/recipe.json recipe.json
# Build dependencies - this is the caching layer
RUN cargo chef cook --release --recipe-path recipe.json
# Build application
COPY Cargo.toml Cargo.lock ./
COPY auth-service ./auth-service
COPY billing-service ./billing-service
COPY document-service ./document-service
COPY genai-service ./genai-service
COPY invoicing-service ./invoicing-service
COPY ledger-service ./ledger-service
COPY notification-service ./notification-service
COPY payment-service ./payment-service
COPY reconciliation-service ./reconciliation-service
COPY service-core ./service-core
COPY proto ./proto
RUN cargo build --release --bin notification-service

# Stage 4: Runtime (The final image)
# Use trixie-slim to match cargo-chef's GLIBC version (2.39)
FROM debian:trixie-slim AS runtime
WORKDIR /app

# Install runtime dependencies
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    libssl3 \
    curl \
    && rm -rf /var/lib/apt/lists/*

# Copy the binary from the builder stage
COPY --from=builder /app/target/release/notification-service /app/notification-service

# Expose the health and gRPC ports
EXPOSE 8080
EXPOSE 8081

# Set healthcheck
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
  CMD curl -f http://localhost:8080/health || exit 1

# Run the binary
CMD ["/app/notification-service"]
