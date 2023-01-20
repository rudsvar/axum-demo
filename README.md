# axum-demo

A web service example with axum.

To start it, you'll first need a database, then you have to run
any missing migrations, and finally run the application itself.
All three steps are listed below.

```shell
docker compose up -d db
sqlx database setup
cargo run
```

You can install `sqlx` with `cargo install sqlx-cli`.
When the application is up and running, visit `localhost:8080`.

# Benchmarks

To discover performance bottlenecks, take a look at https://github.com/flamegraph-rs/flamegraph.
Note that you might have issues installing it in WSL; if so, take a look at https://stackoverflow.com/a/65276025.

```
PERF=/usr/lib/linux-tools/5.4.0-120-generic/perf CARGO_PROFILE_RELEASE_DEBUG=true cargo flamegraph
```
