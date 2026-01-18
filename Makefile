.PHONY: help build test lint fmt check clean install-hooks install uninstall dev backend frontend setup

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

install: ## Install noslop binary to ~/.cargo/bin
	@echo "Building and installing noslop..."
	cargo install --path .
	@echo ""
	@echo "✓ noslop installed successfully!"
	@echo ""
	@echo "You can now run 'noslop' from anywhere."
	@echo "Try: noslop --help"

uninstall: ## Uninstall noslop binary
	@echo "Uninstalling noslop..."
	cargo uninstall noslop
	@echo "✓ noslop uninstalled successfully!"

install-hooks: ## Install noslop globally and set up development hooks
	@echo "Installing noslop binary..."
	@cargo install --path . --force
	@echo ""
	@echo "Installing development git hooks..."
	@mkdir -p .git/hooks
	@echo '#!/bin/sh\n# noslop development pre-commit hook\n# Runs noslop check (via cargo run) + standard Rust checks\n\nset -e\n\necho "Running noslop check (development mode)..."\nif ! cargo run --quiet -- check; then\n    echo "Error: noslop check failed"\n    exit 1\nfi\necho "✓ noslop check passed\\n"\n\n# Run standard Rust checks\nexec ./scripts/pre-commit.sh\n' > .git/hooks/pre-commit
	@chmod +x .git/hooks/pre-commit
	@cp scripts/pre-push.sh .git/hooks/pre-push
	@chmod +x .git/hooks/pre-push
	@echo "✓ noslop installed to ~/.cargo/bin/noslop"
	@echo "✓ Pre-commit hook installed (runs 'cargo run -- check' + Rust tooling)"
	@echo "✓ Pre-push hook installed (includes coverage check)"
	@echo ""
	@echo "Hooks installed successfully!"
	@echo "You can now use 'noslop' command globally."
	@echo "Note: Use SKIP_COVERAGE=1 git push to skip coverage check if needed"

run: ## Run the CLI
	cargo run

watch: ## Watch for changes and rebuild
	cargo watch -x build

doc: ## Generate documentation
	cargo doc --no-deps --open

audit: ## Check dependencies for security vulnerabilities
	cargo audit

coverage: ## Generate code coverage report
	@echo "Generating coverage report..."
	@command -v cargo-tarpaulin >/dev/null 2>&1 || { echo "Installing cargo-tarpaulin..."; cargo install cargo-tarpaulin; }
	cargo tarpaulin --config .tarpaulin.toml

coverage-open: coverage ## Generate and open coverage report in browser
	@if [ -f coverage/index.html ]; then \
		echo "Opening coverage report..."; \
		if command -v xdg-open >/dev/null 2>&1; then \
			xdg-open coverage/index.html; \
		elif command -v open >/dev/null 2>&1; then \
			open coverage/index.html; \
		else \
			echo "Coverage report generated at coverage/index.html"; \
		fi \
	else \
		echo "Coverage report not found"; \
	fi

coverage-check: ## Check if coverage meets minimum thresholds (80% line, 75% branch)
	@echo "Checking coverage thresholds..."
	@./scripts/coverage-check.sh

# Development targets

setup: ## Install all dependencies (npm + cargo)
	cd ui && npm install
	cargo build --features ui

dev: ## Run both backend and frontend dev servers
	@echo "Starting dev servers..."
	@echo "Backend API: http://localhost:9999"
	@echo "Frontend: http://localhost:5173 (use this one)"
	@echo ""
	@trap 'kill $$(jobs -p) 2>/dev/null' EXIT; \
	cargo watch -x 'run --features ui -- ui' & \
	cd ui && npm run dev

backend: ## Run backend with auto-reload on changes
	cargo watch -x 'run --features ui -- ui'

frontend: ## Run frontend dev server with hot-reload
	cd ui && npm run dev

install-watch: ## Install cargo-watch for auto-reload
	cargo install cargo-watch
