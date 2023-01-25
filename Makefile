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

release:
	cargo set-version --bump ${TYPE}
	cargo generate-lockfile
	git add Cargo.toml Cargo.lock
	@TAG=$(shell cargo metadata --format-version 1 | jq '.packages[] | select(.name == "axum-demo") | .version' --raw-output); \
	git commit -m "Update version to $${TAG}"; \
	git tag -a "v$${TAG}" -m "Release v$${TAG}"; \
	git push; \
	git push origin "v$${TAG}"

flamegraph:
	PERF=/usr/lib/linux-tools/5.4.0-120-generic/perf CARGO_PROFILE_RELEASE_DEBUG=true cargo flamegraph
