build:
	cargo build

build-release:
	cargo build --release

run:
	cargo run

run-release:
	cargo run --release

test:
	cargo test

run-release:
	cargo test --release

doc:
	cargo doc

doc-open:
	cargo doc --open

bench:
	# https://crates.io/crates/cargo-criterion
	cargo criterion

sqlx-prepare:
	cargo sqlx prepare -- --lib

sqlx-verify:
	cargo sqlx prepare -- --lib

sqlx-migrate:
	sqlx migrate run

sqlx-reset:
	sqlx database reset
