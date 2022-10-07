FROM lukemathwalker/cargo-chef:latest-rust-1.63-buster AS chef
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
RUN cargo build --release --bin axum-web-demo

FROM debian:buster-slim AS runtime
WORKDIR /app
RUN apt-get update
RUN apt-get install -y libssl-dev
COPY --from=builder /app/target/release/axum-web-demo /usr/local/bin
COPY config.yaml config.yaml
ENV RUST_LOG info,tracing_axum_web=warn,axum_web_demo=debug,sqlx=off
ENTRYPOINT ["/usr/local/bin/axum-web-demo"]
