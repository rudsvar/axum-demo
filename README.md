# axum-demo

A web service example with axum.

To start it, you'll first need a database, then you have to run
any missing migrations, and finally run the application itself.
All three steps are listed below.

```rust
docker compose up -d db
sqlx database setup
cargo run
```

You can install `sqlx` with `cargo install sqlx-cli`.
