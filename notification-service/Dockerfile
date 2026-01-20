# Stage 1: Chef (Prepare)
FROM lukemathwalker/cargo-chef:latest-rust-1.91.0 AS chef
WORKDIR /app

# Stage 2: Planner (Compute recipe)
FROM chef AS planner
COPY Cargo.toml Cargo.lock ./
COPY auth-service ./auth-service
COPY document-service ./document-service
COPY notification-service ./notification-service
COPY payment-service ./payment-service
COPY secure-frontend ./secure-frontend
COPY service-core ./service-core
COPY proto ./proto
RUN cargo chef prepare --recipe-path recipe.json --bin notification-service

# Stage 3: Builder (Build dependencies and binary)
FROM chef AS builder

# Install protobuf compiler for gRPC code generation
RUN apt-get update && apt-get install -y --no-install-recommends \
    protobuf-compiler \
    && rm -rf /var/lib/apt/lists/*

COPY --from=planner /app/recipe.json recipe.json
# Build dependencies - this is the caching layer
RUN cargo chef cook --release --recipe-path recipe.json
# Build application
COPY Cargo.toml Cargo.lock ./
COPY auth-service ./auth-service
COPY document-service ./document-service
COPY notification-service ./notification-service
COPY payment-service ./payment-service
COPY secure-frontend ./secure-frontend
COPY service-core ./service-core
COPY proto ./proto
RUN cargo build --release --bin notification-service

# Stage 4: Runtime (The final image)
FROM debian:bookworm-slim AS runtime
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
EXPOSE 3000
EXPOSE 3001

# Set healthcheck
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
  CMD curl -f http://localhost:3000/health || exit 1

# Run the binary
CMD ["/app/notification-service"]
