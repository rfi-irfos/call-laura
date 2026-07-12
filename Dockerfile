# call-laura / laura-api — Fly-hosted HTTP surface.
# Simple 3-member workspace (unlike ternlang-root's 17-member Dockerfile) — no
# per-crate stub-source layer-caching dance needed, a plain multi-stage build
# is fast enough as-is.

FROM rust:1.86-slim AS builder
RUN apt-get update && apt-get install -y pkg-config libssl-dev && rm -rf /var/lib/apt/lists/*
WORKDIR /build
COPY . .
RUN cargo build --release -p laura-api

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /build/target/release/laura-api /usr/local/bin/laura-api
EXPOSE 8080
CMD ["/usr/local/bin/laura-api"]
