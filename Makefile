.PHONY: help run build install test test/full lint check format

help: ## Show available commands
	@grep -E '^[a-zA-Z/]+:.*?## .*$$' $(MAKEFILE_LIST) | awk 'BEGIN {FS = ":.*?## "}; {printf "  \033[36m%-15s\033[0m %s\n", $$1, $$2}'

run: ## Run harbor CLI interactively (harbor new)
	cargo run --bin harbor -- new

build: ## Build harbor binary (release) → target/release/harbor
	cargo build --release --bin harbor
	@echo "Binary: $(shell pwd)/target/release/harbor"

install: ## Install harbor to ~/.cargo/bin (makes it available system-wide)
	cargo install --path crates/cli

test: ## Fast structural tests
	cargo test --workspace

test/full: ## Slow test: generates a real project, compiles it, runs its internal tests
	cargo test --workspace -- --include-ignored

lint: ## Run clippy with -D warnings
	cargo clippy --workspace -- -D warnings

check: ## cargo check (fast compile check, no binary)
	cargo check --workspace

format: ## Format all code with rustfmt
	cargo fmt --all
