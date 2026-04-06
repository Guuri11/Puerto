.PHONY: run test test/full lint check format

run:
	cargo run --bin harbor

test:
	cargo test --workspace

test/full:
	cargo test --workspace -- --include-ignored

lint:
	cargo clippy --workspace -- -D warnings

check:
	cargo check --workspace

format:
	cargo fmt --all
