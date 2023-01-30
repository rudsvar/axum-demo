FROM lukemathwalker/cargo-chef:latest-rust-1.67-buster AS chef
WORKDIR /app

FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
RUN apt-get update
RUN apt-get install -y protobuf-compiler
COPY --from=planner /app/recipe.json recipe.json
# Build dependencies - this is the caching Docker layer!
RUN cargo chef cook --release --recipe-path recipe.json
# Build application
COPY . .
ENV SQLX_OFFLINE true
RUN cargo build --release --bin axum-demo
# Build docs
RUN cargo doc --release --no-deps

FROM debian:buster-slim AS runtime
WORKDIR /app
RUN apt-get update
RUN apt-get install -y libssl-dev
COPY --from=builder /app/target/release/axum-demo /usr/local/bin
COPY --from=builder /app/target/doc/axum-demo doc
COPY config.toml config.toml
ENV RUST_LOG info,axum_web_demo=debug,sqlx=off
ENTRYPOINT ["/usr/local/bin/axum-demo"]
