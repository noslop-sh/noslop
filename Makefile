.PHONY: help build test lint fmt check clean install-hooks

help: ## Show this help message
	@echo 'Usage: make [target]'
	@echo ''
	@echo 'Available targets:'
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | awk 'BEGIN {FS = ":.*?## "}; {printf "  %-20s %s\n", $$1, $$2}'

build: ## Build the project
	cargo build

build-release: ## Build the project in release mode
	cargo build --release

test: ## Run tests
	cargo test

test-verbose: ## Run tests with verbose output
	cargo test -- --nocapture

lint: ## Run clippy lints
	cargo clippy --all-targets --all-features -- -D warnings

fmt: ## Format code with rustfmt
	cargo fmt --all

fmt-check: ## Check code formatting without modifying files
	cargo fmt --all -- --check

check: ## Run all checks (fmt, clippy, test)
	@echo "Running format check..."
	@make fmt-check
	@echo "\nRunning clippy..."
	@make lint
	@echo "\nRunning tests..."
	@make test
	@echo "\nAll checks passed!"

clean: ## Clean build artifacts
	cargo clean

install-hooks: ## Install git pre-commit hooks
	@echo "Installing pre-commit hooks..."
	@mkdir -p .git/hooks
	@cp scripts/pre-commit.sh .git/hooks/pre-commit
	@chmod +x .git/hooks/pre-commit
	@echo "Pre-commit hooks installed successfully!"

run: ## Run the CLI
	cargo run

watch: ## Watch for changes and rebuild
	cargo watch -x build

doc: ## Generate documentation
	cargo doc --no-deps --open

audit: ## Check dependencies for security vulnerabilities
	cargo audit
