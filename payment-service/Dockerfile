# Runtime image for payment-service
# Requires: docker build -f Dockerfile.builder -t micros-builder . (from repo root)

FROM debian:trixie-slim
WORKDIR /app

# Install runtime dependencies
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    libssl3 \
    curl \
    && rm -rf /var/lib/apt/lists/*

# Copy the pre-built binary from builder image
COPY --from=micros-builder /payment-service /app/payment-service

# Expose the health and gRPC ports
EXPOSE 3003
EXPOSE 3004

# Set healthcheck
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
  CMD curl -f http://localhost:3003/health || exit 1

# Run the binary
CMD ["/app/payment-service"]
