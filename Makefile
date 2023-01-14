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

test-release:
	cargo test --release

doc:
	cargo doc

doc-open:
	cargo doc --open

copy-doc:
	rm -rf doc
	mkdir doc
	cargo doc --no-deps
	cp -r target/doc doc

README.md: src/lib.rs
	cargo readme > $@

bench:
	# https://crates.io/crates/cargo-criterion
	cargo criterion

sqlx-prepare:
	cargo sqlx prepare -- --lib

sqlx-verify:
	cargo sqlx prepare --check -- --lib

sqlx-migrate:
	sqlx migrate run

sqlx-reset:
	sqlx database reset

TYPE=patch

bump:
	cargo bump ${TYPE} --git-tag
