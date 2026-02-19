# Conf-Ops Makefile
# Spec validation and documentation generation targets

.PHONY: validate lint bundle build-docs check-refs help

help: ## Show this help message
	@echo "Conf-Ops Specification Validation"
	@echo ""
	@echo "Available targets:"
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | sort | awk 'BEGIN {FS = ":.*?## "}; {printf "  \033[36m%-15s\033[0m %s\n", $$1, $$2}'

validate: ## Run all spec validations
	@bash scripts/validate-all.sh

lint: ## Lint OpenAPI spec
	@npx @redocly/cli lint docs/api/openapi.yaml

bundle: ## Bundle OpenAPI spec into single file
	@npx @redocly/cli bundle docs/api/openapi.yaml -o docs/api/openapi-bundled.yaml
	@echo "Bundled to docs/api/openapi-bundled.yaml"

build-docs: bundle ## Build HTML documentation
	@npx @redocly/cli build-docs docs/api/openapi-bundled.yaml -o docs/api/index.html
	@echo "Built docs/api/index.html"

check-refs: ## Run cross-reference checks
	@python3 scripts/check-cross-refs.py
