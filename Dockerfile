# ─────────────────────────────────────────────
# Stage 1: Build frontend (SvelteKit → static)
# ─────────────────────────────────────────────
FROM oven/bun:1-alpine AS frontend

WORKDIR /frontend
COPY frontend/package.json frontend/bun.lock* ./
RUN bun install --frozen-lockfile

COPY frontend/ ./
RUN bun run build

# ─────────────────────────────────────────────
# Stage 2: Build Rust binary
# ─────────────────────────────────────────────
FROM rust:1.86-bookworm AS builder

RUN apt-get update && apt-get install -y \
    sqlite3 \
    libsqlite3-dev \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy workspace manifests first for layer caching
COPY Cargo.toml Cargo.lock ./
COPY crates/ ./crates/
COPY migrations/ ./migrations/

# Create a temporary SQLite database so sqlx can validate queries at compile time
RUN sqlite3 ployer.db "" && \
    for f in migrations/*.sql; do sqlite3 ployer.db < "$f"; done

# Copy compiled frontend (needed if we ever switch to rust-embed)
COPY --from=frontend /frontend/build ./frontend/build

RUN DATABASE_URL="sqlite://ployer.db" cargo build --release --bin ployer

# ─────────────────────────────────────────────
# Stage 3: Minimal runtime image
# ─────────────────────────────────────────────
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y \
    ca-certificates \
    libsqlite3-0 \
    libssl3 \
    curl \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Binary
COPY --from=builder /app/target/release/ployer ./ployer

# Frontend static files
COPY --from=frontend /frontend/build ./frontend/build

# Default config (overridden by env vars at runtime)
COPY config/ ./config/

# SQLite data lives on a mounted volume
VOLUME ["/data"]

EXPOSE 3001

ENV PLOYER_DATABASE_URL="sqlite:///data/ployer.db?mode=rwc" \
    FRONTEND_DIR="/app/frontend/build"

HEALTHCHECK --interval=30s --timeout=5s --start-period=10s --retries=3 \
    CMD curl -f http://localhost:3001/api/v1/health || exit 1

CMD ["./ployer", "start"]
