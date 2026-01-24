# Centralized builder for all microservices
# Builds all service binaries in a single image to avoid memory exhaustion
# from parallel builds of the full workspace.
#
# Usage:
#   docker build -f Dockerfile.builder -t micros-builder .
#   docker compose build  # Service Dockerfiles copy from micros-builder

# Stage 1: Chef (Prepare)
FROM lukemathwalker/cargo-chef:latest-rust-1.91.0 AS chef
WORKDIR /app

# Install protobuf compiler and dev files (includes google well-known types)
RUN apt-get update && apt-get install -y --no-install-recommends \
    protobuf-compiler \
    libprotobuf-dev \
    && rm -rf /var/lib/apt/lists/*

# Stage 2: Planner (Compute recipe for all services)
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
RUN cargo chef prepare --recipe-path recipe.json

# Stage 3: Builder (Build all binaries)
FROM chef AS builder
COPY --from=planner /app/recipe.json recipe.json

# Build dependencies - this is the caching layer
RUN cargo chef cook --release --recipe-path recipe.json

# Copy full source
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

# Build all service binaries in one compilation
RUN cargo build --release \
    --bin auth-service \
    --bin billing-service \
    --bin document-service \
    --bin genai-service \
    --bin invoicing-service \
    --bin ledger-service \
    --bin notification-service \
    --bin payment-service \
    --bin reconciliation-service

# Final stage just exposes the binaries for COPY --from
FROM scratch AS binaries
COPY --from=builder /app/target/release/auth-service /
COPY --from=builder /app/target/release/billing-service /
COPY --from=builder /app/target/release/document-service /
COPY --from=builder /app/target/release/genai-service /
COPY --from=builder /app/target/release/invoicing-service /
COPY --from=builder /app/target/release/ledger-service /
COPY --from=builder /app/target/release/notification-service /
COPY --from=builder /app/target/release/payment-service /
COPY --from=builder /app/target/release/reconciliation-service /
