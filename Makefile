.PHONY: help run build install test test/full lint check format setup

help: ## Show available commands
	@grep -E '^[a-zA-Z/]+:.*?## .*$$' $(MAKEFILE_LIST) | awk 'BEGIN {FS = ":.*?## "}; {printf "  \033[36m%-15s\033[0m %s\n", $$1, $$2}'

run: ## Run harbor CLI interactively (harbor new)
	cargo run --bin harbor -- new

build: ## Build harbor binary (release) → target/release/harbor
	cargo build --release --bin harbor
	@echo "Binary: $(shell pwd)/target/release/harbor"

install: ## Install harbor to ~/.cargo/bin (makes it available system-wide)
	cargo install --path crates/cli

setup: ## Install required dev tools (run once after cloning)
	cargo install cargo-nextest --locked

test: ## Fast structural tests (requires: make setup)
	cargo nextest run --workspace

test/full: ## Slow test: generates a real project, compiles it, runs its internal tests
	cargo nextest run --workspace --run-ignored all

lint: ## Run clippy with -D warnings
	cargo clippy --workspace -- -D warnings

check: ## cargo check (fast compile check, no binary)
	cargo check --workspace

format: ## Format all code with rustfmt
	cargo fmt --all
