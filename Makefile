.PHONY: help run build install setup test test/full test/db test/kafka test/mutation test/snapshots lint check format format/fix audit audit/fix

CARGO := cargo

GREEN  := \033[1;32m
YELLOW := \033[1;33m
CYAN   := \033[1;36m
RED    := \033[1;31m
NC     := \033[0m

help: ## Show available commands
	@echo ""
	@echo "${CYAN}Available targets:${NC}"
	@echo ""
	@grep -E '^[a-zA-Z/]+:.*?## .*$$' $(MAKEFILE_LIST) | awk 'BEGIN {FS = ":.*?## "}; {printf "  ${YELLOW}%-20s${NC} %s\n", $$1, $$2}'
	@echo ""

run: ## Run puerto CLI interactively (puerto new)
	@echo "${GREEN}Running puerto CLI...${NC}"
	$(CARGO) run --bin puerto -- new

build: ## Build puerto binary (release) → target/release/puerto
	@echo "${GREEN}Building puerto...${NC}"
	$(CARGO) build --release --bin puerto
	@echo "Binary: $(shell pwd)/target/release/puerto"

install: ## Install puerto to ~/.cargo/bin
	@echo "${GREEN}Installing puerto...${NC}"
	$(CARGO) install --path crates/cli

setup: ## Install required dev tools (run once after cloning)
	@echo "${CYAN}Setting up development environment...${NC}"
	$(CARGO) install cargo-nextest --locked
	$(CARGO) install cargo-mutants --locked

test: ## Fast structural tests
	@echo "${YELLOW}Running structural tests...${NC}"
	$(CARGO) nextest run --workspace

test/full: ## Slow test: generates real projects, compiles them, runs their internal tests
	@echo "${YELLOW}Running compile matrix (generates + compiles + tests real projects)...${NC}"
	$(CARGO) nextest run --workspace --run-ignored all --test-threads 3

test/snapshots: ## Re-record the generated-output snapshots after an intended change — then read the .snap diff
	INSTA_UPDATE=always $(CARGO) nextest run --workspace -E 'test(/^tests::generated_output::/)'

test/db: ## Slow test: generated --db project against a throwaway Postgres (needs Docker + sqlx-cli)
	@echo "${YELLOW}Starting throwaway Postgres on :55432...${NC}"
	@docker run -d --rm --name puerto-test-pg -e POSTGRES_PASSWORD=pg -p 55432:5432 postgres:16-alpine >/dev/null
	@until docker exec puerto-test-pg pg_isready -U postgres -q; do sleep 1; done
	PUERTO_TEST_DATABASE_URL=postgres://postgres:pg@localhost:55432 \
		$(CARGO) nextest run --workspace --run-ignored all -E 'test(postgres)'; \
		status=$$?; docker stop puerto-test-pg >/dev/null; exit $$status

test/kafka: ## Slow test: the Kafka transport against a throwaway Redpanda (+ Postgres for the generated worker)
	@echo "${YELLOW}Starting throwaway Redpanda on :59092 and Postgres on :55433...${NC}"
	@docker run -d --rm --name puerto-test-redpanda -p 59092:59092 docker.redpanda.com/redpandadata/redpanda:v24.2.7 \
		redpanda start --mode dev-container --smp 1 --kafka-addr PLAINTEXT://0.0.0.0:59092 \
		--advertise-kafka-addr PLAINTEXT://localhost:59092 >/dev/null
	@docker run -d --rm --name puerto-test-pg-kafka -e POSTGRES_PASSWORD=pg -p 55433:5432 postgres:16-alpine >/dev/null
	@until docker exec puerto-test-redpanda rpk cluster health -X brokers=localhost:59092 2>/dev/null | grep -q 'Healthy:.*true'; do sleep 1; done
	@until docker exec puerto-test-pg-kafka pg_isready -U postgres -q; do sleep 1; done
	PUERTO_TEST_KAFKA_BROKERS=localhost:59092 PUERTO_TEST_DATABASE_URL=postgres://postgres:pg@localhost:55433 \
		$(CARGO) nextest run --workspace --run-ignored all -E 'test(/(^|::)kafka_/)'; \
		status=$$?; docker stop puerto-test-redpanda puerto-test-pg-kafka >/dev/null; exit $$status

test/mutation: ## Slow: mutation testing on Puerto's pure logic (see .cargo/mutants.toml)
	@echo "${YELLOW}Running cargo-mutants...${NC}"
	@mkdir -p target/mutants-tmp
	# Each job copies the tree and builds it: on a tmpfs /tmp that runs out of space mid-link.
	TMPDIR=$(CURDIR)/target/mutants-tmp $(CARGO) mutants --workspace --jobs 3

lint: ## Run clippy with -D warnings
	@echo "${CYAN}Linting code...${NC}"
	$(CARGO) clippy --workspace --all-targets -- -D warnings

check: ## Fast compile check (no binary)
	@echo "${CYAN}Checking code...${NC}"
	$(CARGO) check --workspace

format: ## Check code formatting
	@echo "${CYAN}Checking code formatting...${NC}"
	$(CARGO) fmt --all -- --check

format/fix: ## Fix code formatting
	@echo "${CYAN}Fixing code formatting...${NC}"
	$(CARGO) fmt --all

audit: ## Run security audit on dependencies
	@echo "${CYAN}Running security audit...${NC}"
	$(CARGO) audit

audit/fix: ## Preview security fixes (dry-run)
	@echo "${CYAN}Previewing security fixes (dry-run)...${NC}"
	$(CARGO) audit fix --dry-run
