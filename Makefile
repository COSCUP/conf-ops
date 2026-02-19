# Conf-Ops Makefile

.PHONY: help dev-up dev-down migrate lint lint-backend lint-frontend lint-spec \
        test test-backend test-frontend validate generate-openapi build-docs check-refs

help: ## Show this help message
	@echo "Conf-Ops Development Commands"
	@echo ""
	@echo "Available targets:"
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | sort | awk 'BEGIN {FS = ":.*?## "}; {printf "  \033[36m%-20s\033[0m %s\n", $$1, $$2}'

# --- Development Environment ---

dev-up: ## Start development services (PostgreSQL + MailHog)
	@docker compose -f docker-compose.dev.yml up -d

dev-down: ## Stop development services
	@docker compose -f docker-compose.dev.yml down

migrate: ## Run database migrations
	@cargo run -- migrate

# --- Linting ---

lint: lint-backend lint-frontend lint-spec ## Run all lints

lint-backend: ## Lint backend (clippy + fmt)
	cargo clippy -- -D warnings
	cargo fmt -- --check

lint-frontend: ## Lint frontend
	cd frontend && pnpm run lint

lint-spec: ## Lint OpenAPI spec
	@npx @redocly/cli lint docs/api/openapi.yaml

# --- Testing ---

test: test-backend test-frontend ## Run all tests

test-backend: ## Run backend tests
	cargo test

test-frontend: ## Run frontend tests
	cd frontend && pnpm run test -- --run

# --- Spec Tools ---

validate: ## Run all spec validations
	@bash scripts/validate-all.sh

generate-openapi: ## Generate OpenAPI spec from utoipa annotations
	@cargo xtask generate-openapi
	@echo "Generated docs/api/openapi-generated.yaml"

build-docs: generate-openapi ## Build HTML documentation
	@npx @redocly/cli build-docs docs/api/openapi-generated.yaml -o docs/api/index.html
	@echo "Built docs/api/index.html"

check-refs: ## Run cross-reference checks
	@python3 scripts/check-cross-refs.py
