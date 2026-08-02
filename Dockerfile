FROM rust:1-slim AS builder
WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY src ./src
RUN cargo build --release

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/swarm-arena-server /usr/local/bin/swarm-arena-server
# Render's Docker runtime always sets PORT=10000 and routes to that exposed
# port - EXPOSE 8080 here silently broke routing even though the app itself
# correctly bound to whatever $PORT actually was (confirmed in Render logs:
# "listening on :10000" while every request still 404'd at the edge).
EXPOSE 10000
CMD ["swarm-arena-server"]
