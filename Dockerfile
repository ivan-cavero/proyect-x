# Build stage
FROM rust:nightly-slim AS builder

WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY crates ./crates

RUN cargo build --release --bin project-x

# Runtime stage
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/project-x /usr/local/bin/

RUN mkdir -p /data /config

EXPOSE 8080

ENTRYPOINT ["project-x"]
CMD ["dashboard"]