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
COPY document-service ./document-service
COPY notification-service ./notification-service
COPY payment-service ./payment-service
COPY secure-frontend ./secure-frontend
COPY service-core ./service-core
COPY proto ./proto
RUN cargo chef prepare --recipe-path recipe.json --bin secure-frontend

# Stage 3: Builder (Build dependencies and binary)
FROM chef AS builder
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
RUN cargo build --release --bin secure-frontend

# Stage 4: CSS Builder (Build optimized CSS with Tailwind)
FROM node:20-slim AS css-builder
WORKDIR /app
COPY secure-frontend/package.json secure-frontend/package-lock.json* ./
RUN npm install
COPY secure-frontend/tailwind.config.js ./
COPY secure-frontend/templates ./templates
COPY secure-frontend/static/input.css ./static/input.css
RUN npm run build:css

# Stage 5: Runtime (The final image)
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
COPY --from=builder /app/target/release/secure-frontend /usr/local/bin/secure-frontend

# Copy static assets and templates if they are not embedded
# Askama embeds templates, but static files might be needed
RUN mkdir -p /app/secure-frontend/static
COPY --from=css-builder /app/static/output.css /app/secure-frontend/static/output.css
COPY --from=builder /app/secure-frontend/config /app/secure-frontend/config

# Expose the service port
EXPOSE 8080

# Run the binary
CMD ["secure-frontend"]
