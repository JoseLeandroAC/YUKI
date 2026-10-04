# ==============================================================================
# YUKI RUNTIME — PRODUCTION OCI CONTAINER (MVP-1 / MARCO 4)
# Multi-stage minimal footprint, non-root, hardened execution environment.
# ==============================================================================

# ------------------------------------------------------------------------------
# Stage 1: Builder
# ------------------------------------------------------------------------------
FROM rust:1.98-bookworm AS builder

RUN apt-get update && apt-get install -y --no-install-recommends \
    pkg-config \
    libssl-dev \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /usr/src/yuki

# Copy dependency manifests and source
COPY Cargo.toml Cargo.lock ./
COPY src/ ./src/

# Compile release binary
RUN cargo build --release --bin yuki

# ------------------------------------------------------------------------------
# Stage 2: Minimal Runtime
# ------------------------------------------------------------------------------
FROM debian:bookworm-slim

# Install minimal runtime dependencies only (no build tools, compilers, or headers)
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

# Non-root user and group creation (UID/GID 10001)
RUN groupadd -g 10001 yuki && \
    useradd -u 10001 -g yuki -s /bin/false -M yuki

# Persistent storage mount point owned by yuki:yuki with restrictive mode 0700
RUN mkdir -p /var/lib/yuki && \
    chown -R yuki:yuki /var/lib/yuki && \
    chmod 0700 /var/lib/yuki

# Copy binary from builder with immutable read/execute-only permissions
COPY --from=builder --chown=root:root /usr/src/yuki/target/release/yuki /usr/local/bin/yuki
RUN chmod 0555 /usr/local/bin/yuki

USER yuki:yuki
WORKDIR /var/lib/yuki

ENV YUKI_DATA_DIR=/var/lib/yuki \
    YUKI_PERSISTENCE_PATH=/var/lib/yuki/audit.db \
    YUKI_ENVIRONMENT=production \
    RUST_LOG=info

VOLUME ["/var/lib/yuki"]

ENTRYPOINT ["/usr/local/bin/yuki"]
CMD ["health"]
