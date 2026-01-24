# Runtime image for document-service
# Requires: docker build -f Dockerfile.builder -t micros-builder . (from repo root)

FROM debian:trixie-slim
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

# Copy the pre-built binary from builder image
COPY --from=micros-builder /document-service /app/document-service

# Expose health and gRPC ports
EXPOSE 8080 8081

# Set healthcheck
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
  CMD curl -f http://localhost:8080/health || exit 1

# Run the binary
CMD ["/app/document-service"]
