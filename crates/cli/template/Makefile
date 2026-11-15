.PHONY: help setup run \
        test test/domain test/application test/infrastructure \
        lint format format/fix check clean clean/incremental \
        audit audit/fix \
        generate/entity generate/use-case generate/bootstrap \
        web/install web/require-install web/dev web/test web/lint web/sync

CARGO := cargo
BUN   := bun

# The escape byte itself, not `\033`: `echo` does not interpret escapes where `sh` is bash.
GREEN  := $(shell printf '\033[1;32m')
YELLOW := $(shell printf '\033[1;33m')
CYAN   := $(shell printf '\033[1;36m')
RED    := $(shell printf '\033[1;31m')
NC     := $(shell printf '\033[0m')

help: ## Show available commands
	@echo ""
	@echo "${CYAN}Available targets:${NC}"
	@echo ""
	@grep -E '^[a-zA-Z/_-]+:.*?## .*$$' $(MAKEFILE_LIST) | awk 'BEGIN {FS = ":.*?## "}; {printf "  ${YELLOW}%-32s${NC} %s\n", $$1, $$2}'
	@echo ""

setup: ## Install required dev tools (run once after cloning)
	@echo "${CYAN}Setting up development environment...${NC}"
	$(CARGO) install cargo-nextest --locked
	$(CARGO) install cargo-audit --locked

run: ## Run the API server (http://localhost:8080)
	@echo "${GREEN}Starting API server (in memory: data is lost on restart)...${NC}"
	$(CARGO) run -p api

test: ## Run all tests
	@echo "${YELLOW}Running all tests...${NC}"
	$(CARGO) nextest run --workspace
	@if [ -d web ]; then $(MAKE) web/test; fi

test/domain: ## Run the tests of every domain crate (dominio_*)
	@echo "${YELLOW}Running domain tests...${NC}"
	$(CARGO) nextest run --workspace -E 'package(dominio_*)' --no-tests=warn

test/application: ## Run the tests of every application crate (aplicacion_*)
	@echo "${YELLOW}Running application tests...${NC}"
	$(CARGO) nextest run --workspace -E 'package(aplicacion_*)' --no-tests=warn

test/infrastructure: ## Run the tests of every infrastructure crate (infra_*; requires DB with --db)
	@echo "${YELLOW}Running infrastructure tests...${NC}"
	$(CARGO) nextest run --workspace -E 'package(infra_*)' --no-tests=warn

lint: ## Clippy with -D warnings
	@echo "${CYAN}Linting code...${NC}"
	$(CARGO) clippy --workspace --all-targets -- -D warnings
	@if [ -d web ]; then $(MAKE) web/lint; fi

format: ## Check code formatting
	@echo "${CYAN}Checking code formatting...${NC}"
	$(CARGO) fmt --all -- --check

format/fix: ## Fix code formatting
	@echo "${CYAN}Fixing code formatting...${NC}"
	$(CARGO) fmt --all

check: ## Fast compile check (no binary)
	@echo "${CYAN}Checking code...${NC}"
	$(CARGO) check --workspace

clean: ## Remove build artifacts
	@echo "${RED}Cleaning build artifacts...${NC}"
	$(CARGO) clean

clean/incremental: ## Remove the incremental caches (they grow fast with one crate per context)
	@echo "${RED}Removing incremental caches...${NC}"
	rm -rf target/debug/incremental target/release/incremental

audit: ## Run security audit on dependencies
	@echo "${CYAN}Running security audit...${NC}"
	$(CARGO) audit

audit/fix: ## Preview security fixes (dry-run)
	@echo "${CYAN}Previewing security fixes (dry-run)...${NC}"
	$(CARGO) audit fix --dry-run

generate/entity: ## Scaffold a new DDD entity — make generate/entity NAME=Product
	@if [ -z "$$NAME" ]; then \
		echo "${RED}Error: provide entity name — make generate/entity NAME=Product${NC}"; \
		exit 1; \
	fi
	@echo "${GREEN}Scaffolding entity '$$NAME'...${NC}"
	puerto generar scaffold $$NAME

generate/use-case: ## Add a use case to an entity — make generate/use-case ENTITY=Product ACTION=delete_product
	@if [ -z "$$ENTITY" ] || [ -z "$$ACTION" ]; then \
		echo "${RED}Error: provide entity and action — make generate/use-case ENTITY=Product ACTION=delete_product${NC}"; \
		exit 1; \
	fi
	@echo "${GREEN}Adding use case '$$ACTION' to '$$ENTITY'...${NC}"
	puerto generar caso-de-uso $$ENTITY $$ACTION

generate/bootstrap: ## Regenerate the wiring (cableado, bootstrap.rs) from puerto.toml
	@echo "${GREEN}Regenerating bootstrap.rs...${NC}"
	puerto generar bootstrap

web/install: ## Install the frontend dependencies (needs Bun)
	@echo "${CYAN}Installing frontend dependencies...${NC}"
	cd web && $(BUN) install

# Not in `make help`: the prerequisite of every target that runs the frontend's tools.
web/require-install:
	@if [ ! -d web/node_modules ]; then \
		echo "${RED}web/node_modules is missing — run: make web/install${NC}"; \
		exit 1; \
	fi

web/dev: web/require-install ## Run the frontend dev server (http://localhost:5173, /api proxied to :8080)
	@echo "${GREEN}Starting frontend dev server...${NC}"
	cd web && $(BUN) run dev

web/test: web/require-install ## Run the frontend tests
	@echo "${YELLOW}Running frontend tests...${NC}"
	cd web && $(BUN) test

web/lint: web/require-install ## Typecheck, lint and check the frontend Dependency Rule
	@echo "${CYAN}Linting frontend...${NC}"
	cd web && $(BUN) run typecheck && $(BUN) run lint && $(BUN) run lint:arch

web/sync: web/require-install ## After a scaffold or a DTO change: export the OpenAPI spec, regenerate the web types
	@echo "${CYAN}Exporting docs/api/openapi.json...${NC}"
	UPDATE_OPENAPI=1 $(CARGO) test -p api openapi
	@echo "${CYAN}Generating web/src/infrastructure/http/schema.d.ts...${NC}"
	cd web && $(BUN) run openapi:types
	@echo "${CYAN}Formatting web/ (Biome, no lint fixes)...${NC}"
	cd web && $(BUN) run format
