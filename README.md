# axum-demo

A web service example with axum.

To start it, you'll first need a database, then you have to run
any missing migrations, and finally run the application itself.
All three steps are listed below.

```shell
docker compose up -d
sqlx database setup
cargo run
```

You can install `sqlx` with `cargo install sqlx-cli`.
When the application is up and running, visit `localhost:8080`.


# Docker

Running the application with `docker compose`.

```
# Run a single instance with nginx as a proxy
docker-compose up --build nginx axum-demo
# Run multiple instances wihh nginx as a load balancer
docker-compose up --build nginx axum-demo --scale axum-demo=10
```

# Swarm

You can run the entire stack with swarm,

```
docker compose build axum-demo
docker stack deploy mystack --compose-file stack.yml
docker service ls
docker service logs mystack_axum-demo --tail 0 --follow
```

and remove it with

```
docker stack rm mystack
```

# Benchmarks

To discover performance bottlenecks, take a look at https://github.com/flamegraph-rs/flamegraph.
Note that you might have issues installing it in WSL; if so, take a look at https://stackoverflow.com/a/65276025.

```
PERF=/usr/lib/linux-tools/5.4.0-120-generic/perf CARGO_PROFILE_RELEASE_DEBUG=true cargo flamegraph
```
