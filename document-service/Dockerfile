# Stage 1: Chef (Prepare)
FROM lukemathwalker/cargo-chef:latest-rust-1.91.0 AS chef
WORKDIR /app

# Install protobuf compiler for gRPC
RUN apt-get update && apt-get install -y protobuf-compiler && rm -rf /var/lib/apt/lists/*

# Stage 2: Planner (Compute recipe)
FROM chef AS planner
COPY Cargo.toml Cargo.lock ./
COPY auth-service ./auth-service
COPY document-service ./document-service
COPY secure-frontend ./secure-frontend
COPY service-core ./service-core
COPY notification-service ./notification-service
COPY proto ./proto
RUN cargo chef prepare --recipe-path recipe.json --bin document-service

# Stage 3: Builder (Build dependencies and binary)
FROM chef AS builder
COPY --from=planner /app/recipe.json recipe.json
# Build dependencies - this is the caching layer
RUN cargo chef cook --release --recipe-path recipe.json
# Build application
COPY Cargo.toml Cargo.lock ./
COPY auth-service ./auth-service
COPY document-service ./document-service
COPY secure-frontend ./secure-frontend
COPY service-core ./service-core
COPY notification-service ./notification-service
COPY proto ./proto
RUN cargo build --release --bin document-service

# Stage 4: Runtime (The final image)
FROM debian:bookworm-slim AS runtime
WORKDIR /app

# Install runtime dependencies including FFmpeg, Poppler, and ImageMagick
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    libssl3 \
    curl \
    ffmpeg \
    poppler-utils \
    imagemagick \
    libwebp-dev \
    && rm -rf /var/lib/apt/lists/*

# Copy the binary from the builder stage
COPY --from=builder /app/target/release/document-service /app/document-service

# Expose health and gRPC ports
EXPOSE 8080 8081

# Set healthcheck
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
  CMD curl -f http://localhost:8080/health || exit 1

# Run the binary
CMD ["/app/document-service"]
