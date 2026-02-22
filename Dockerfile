# ── Stage 1: Rust builder ────────────────────────────────────
FROM rust:1.82-bookworm AS rust-builder

WORKDIR /app

# Copy Cargo files first for dependency caching
COPY Cargo.toml Cargo.lock ./
COPY xtask/ xtask/

# Create dummy source to build dependencies
RUN mkdir src && echo "fn main() {}" > src/main.rs && \
    echo "pub fn lib() {}" > src/lib.rs
RUN cargo build --release 2>/dev/null || true
RUN rm -rf src

# Copy actual source and build
COPY src/ src/
COPY migrations/ migrations/
COPY .sqlx/ .sqlx/

ENV SQLX_OFFLINE=true
RUN touch src/main.rs src/lib.rs && cargo build --release

# ── Stage 2: Node.js builder (frontend) ────────────────────
FROM node:20-slim AS frontend-builder

WORKDIR /app/frontend

# Install pnpm
RUN corepack enable && corepack prepare pnpm@latest --activate

# Copy package files for dependency caching
COPY frontend/package.json frontend/pnpm-lock.yaml ./

RUN pnpm install --frozen-lockfile

# Copy frontend source and build
COPY frontend/ ./
COPY docs/api/openapi-generated.yaml /app/docs/api/openapi-generated.yaml

RUN pnpm run build-only

# ── Stage 3: Runtime ────────────────────────────────────────
FROM debian:bookworm-slim

RUN apt-get update && \
    apt-get install -y --no-install-recommends ca-certificates curl && \
    rm -rf /var/lib/apt/lists/*

# Create non-root user
RUN groupadd -r confops && useradd -r -g confops confops

# Copy binary
COPY --from=rust-builder /app/target/release/conf-ops /usr/local/bin/confops

# Copy migrations
COPY --from=rust-builder /app/migrations /app/migrations

# Copy frontend build output
COPY --from=frontend-builder /app/frontend/dist /app/static

# Create file storage directory
RUN mkdir -p /data/files && chown -R confops:confops /data/files

WORKDIR /app

USER confops

EXPOSE 8080

HEALTHCHECK --interval=30s --timeout=5s --retries=3 --start-period=10s \
    CMD curl -f http://localhost:8080/healthz || exit 1

ENTRYPOINT ["confops"]
