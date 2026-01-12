# syntax=docker/dockerfile:1
# .dockerignore hints:
#   target/
#   web/admin/node_modules/
#   web/admin-dist/
#   .git/
#   .gitignore
#   .gitlab-ci.yml
#   docker-compose.yml
#   docker.compose
#   *.md

# ─────────────────────────────────────────────
# Stage 1: builder
# ─────────────────────────────────────────────
FROM rust:1-bookworm AS builder

# Install Node.js 20 via NodeSource
RUN apt-get update && apt-get install -y --no-install-recommends \
        ca-certificates \
        curl \
        gnupg \
    && curl -fsSL https://deb.nodesource.com/setup_20.x | bash - \
    && apt-get install -y --no-install-recommends nodejs \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy the entire project
COPY . .

# Build the frontend
RUN cd web/admin \
    && npm ci \
    && npm run build

# Build the Rust binary in release mode
RUN cargo build --release --locked

# ─────────────────────────────────────────────
# Stage 2: runtime
# ─────────────────────────────────────────────
FROM debian:bookworm-slim AS runtime

RUN apt-get update && apt-get install -y --no-install-recommends \
        ca-certificates \
        libssl3 \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy compiled binary from builder
COPY --from=builder /app/target/release/kaxapage ./kaxapage

# Copy database migrations
COPY --from=builder /app/migrations ./migrations

EXPOSE 8080

ENV RUST_LOG=info

RUN useradd -r -s /bin/false kaxapage
USER kaxapage

CMD ["./kaxapage"]
