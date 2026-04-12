.PHONY: help run test lint format setup

help: ## Show available commands
	@grep -E '^[a-zA-Z/]+:.*?## .*$$' $(MAKEFILE_LIST) | awk 'BEGIN {FS = ":.*?## "}; {printf "  \033[36m%-15s\033[0m %s\n", $$1, $$2}'

run: ## Run the API server (http://localhost:8080)
	cargo run -p {{project-name}}

test: ## Run all tests
	cargo nextest run --workspace

lint: ## Clippy with -D warnings
	cargo clippy --workspace -- -D warnings

format: ## Format all code
	cargo fmt --all

setup: ## Install required dev tools (run once after cloning)
	cargo install cargo-nextest --locked
