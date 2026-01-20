# Stage 1: Chef (Prepare)
FROM lukemathwalker/cargo-chef:latest-rust-1.91.0 AS chef
WORKDIR /app

# Install protobuf compiler for proto compilation
RUN apt-get update && apt-get install -y --no-install-recommends protobuf-compiler && rm -rf /var/lib/apt/lists/*

# Stage 2: Planner (Compute recipe)
FROM chef AS planner
COPY Cargo.toml Cargo.lock ./
COPY proto ./proto
COPY auth-service ./auth-service
COPY document-service ./document-service
COPY secure-frontend ./secure-frontend
COPY service-core ./service-core
RUN cargo chef prepare --recipe-path recipe.json --bin auth-service

# Stage 3: Builder (Build dependencies and binary)
FROM chef AS builder
COPY --from=planner /app/recipe.json recipe.json
# Build dependencies - this is the caching layer
RUN cargo chef cook --release --recipe-path recipe.json
# Build application
COPY Cargo.toml Cargo.lock ./
COPY proto ./proto
COPY auth-service ./auth-service
COPY document-service ./document-service
COPY secure-frontend ./secure-frontend
COPY service-core ./service-core
RUN cargo build --release --bin auth-service

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
COPY --from=builder /app/target/release/auth-service /app/auth-service

# Expose ports: HTTP health (3000) and gRPC (3001)
EXPOSE 3000
EXPOSE 3001

# Set healthcheck on HTTP health endpoint
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
  CMD curl -f http://localhost:3000/health || exit 1

# Run the binary
CMD ["/app/auth-service"]
