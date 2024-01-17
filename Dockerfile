FROM lukemathwalker/cargo-chef:latest-rust-alpine3.19 AS chef
WORKDIR /app

FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
COPY --from=planner /app/recipe.json recipe.json
# Build dependencies - this is the caching Docker layer!
RUN cargo chef cook --release --recipe-path recipe.json
# Build application
COPY . .
ENV SQLX_OFFLINE true
RUN cargo build --release --bin axum-demo
# Build docs
RUN cargo doc --no-deps --release

FROM alpine:3.19 AS runtime
WORKDIR /app
COPY --from=builder /app/target/release/axum-demo /usr/local/bin
COPY --from=builder /app/target/doc doc
COPY config.toml config.toml
EXPOSE 80
ENV RUST_LOG warn,axum_demo=debug
ENV APP__SERVER__HTTP_PORT 80
ENTRYPOINT ["/usr/local/bin/axum-demo"]
