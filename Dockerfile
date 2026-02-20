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
# Stage 2: cargo-chef planner
# ─────────────────────────────────────────────
FROM rust:1.88-alpine AS chef

RUN apk add --no-cache sqlite sqlite-dev pkgconfig openssl-dev musl-dev
RUN --network=host cargo install cargo-chef --locked

WORKDIR /app

# ─────────────────────────────────────────────
# Stage 3: Generate dependency recipe
# ─────────────────────────────────────────────
FROM chef AS planner

COPY Cargo.toml Cargo.lock ./
COPY crates/ ./crates/
RUN cargo chef prepare --recipe-path recipe.json

# ─────────────────────────────────────────────
# Stage 4: Build dependencies only (cached)
# ─────────────────────────────────────────────
FROM chef AS builder

COPY --from=planner /app/recipe.json recipe.json
# This layer is cached as long as Cargo.toml/Cargo.lock don't change
RUN --network=host cargo chef cook --release --recipe-path recipe.json

# Now copy source and build only your code
COPY Cargo.toml Cargo.lock ./
COPY crates/ ./crates/
COPY migrations/ ./migrations/

# Create a temporary SQLite database so sqlx can validate queries at compile time
RUN sqlite3 ployer.db "" && \
    for f in migrations/*.sql; do sqlite3 ployer.db < "$f"; done

COPY --from=frontend /frontend/build ./frontend/build

# Use dynamic linking on musl/Alpine so shared OpenSSL libs are found
ENV RUSTFLAGS="-C target-feature=-crt-static"

RUN --network=host DATABASE_URL="sqlite://ployer.db" cargo build --release --bin ployer

# ─────────────────────────────────────────────
# Stage 5: Minimal runtime image
# ─────────────────────────────────────────────
FROM alpine:3

RUN apk add --no-cache \
    ca-certificates \
    sqlite-libs \
    openssl \
    curl

WORKDIR /app

COPY --from=builder /app/target/release/ployer ./ployer
COPY --from=frontend /frontend/build ./frontend/build
COPY config/ ./config/

VOLUME ["/data"]

EXPOSE 3001

ENV PLOYER_DATABASE_URL="sqlite:///data/ployer.db?mode=rwc" \
    FRONTEND_DIR="/app/frontend/build"

HEALTHCHECK --interval=30s --timeout=5s --start-period=10s --retries=3 \
    CMD curl -f http://localhost:3001/api/v1/health || exit 1

CMD ["./ployer", "start"]
