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
FROM rust:1.88-alpine AS builder

RUN apk add --no-cache \
    sqlite \
    sqlite-dev \
    pkgconfig \
    openssl-dev \
    musl-dev

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

RUN --network=host DATABASE_URL="sqlite://ployer.db" cargo build --release --bin ployer

# ─────────────────────────────────────────────
# Stage 3: Minimal runtime image
# ─────────────────────────────────────────────
FROM alpine:3

RUN apk add --no-cache \
    ca-certificates \
    sqlite-libs \
    openssl \
    curl

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
