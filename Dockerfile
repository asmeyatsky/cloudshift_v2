# Stage 1: Build frontend
FROM node:20-slim AS frontend
WORKDIR /app/ui
COPY ui/package.json ui/package-lock.json* ./
RUN npm install
COPY ui/ .
RUN npm run build

# Stage 2: Build Rust server
FROM rust:1-bookworm AS builder
WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY crates crates
RUN cargo build --release -p cloudshift-server

# Stage 3: Runtime
FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/cloudshift-server /usr/local/bin/
COPY --chown=nobody:nogroup patterns /app/patterns
COPY --from=frontend --chown=nobody:nogroup /app/static /app/static
ENV PORT=8080
ENV CLOUDSHIFT_PATTERNS_DIR=/app/patterns
ENV CLOUDSHIFT_STATIC_DIR=/app/static
EXPOSE 8080
USER nobody
CMD ["cloudshift-server"]
