.PHONY: help build test test-unit test-adapter test-integration test-lib lint fmt check clean install-hooks install uninstall dev dev-setup dev-teardown

help: ## Show this help message
	@echo 'Usage: make [target]'
	@echo ''
	@echo 'Available targets:'
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | awk 'BEGIN {FS = ":.*?## "}; {printf "  %-20s %s\n", $$1, $$2}'

build: ## Build the project
	cargo build

build-release: ## Build the project in release mode
	cargo build --release

test: ## Run all tests
	cargo test

test-unit: ## Run unit tests only (tests/unit/)
	cargo test --test unit

test-adapter: ## Run adapter tests only (tests/adapter/)
	cargo test --test adapter

test-integration: ## Run integration tests only (tests/integration/)
	cargo test --test integration

test-lib: ## Run library tests only (inline tests in src/)
	cargo test --lib

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

dev-setup: ## Set up development symlink (points global noslop to debug binary)
	@echo "Setting up development environment..."
	@cargo build
	@if [ -L "$(HOME)/.cargo/bin/noslop" ] || [ -e "$(HOME)/.cargo/bin/noslop" ]; then \
		echo "Backing up existing noslop to ~/.cargo/bin/noslop.bak"; \
		mv "$(HOME)/.cargo/bin/noslop" "$(HOME)/.cargo/bin/noslop.bak"; \
	fi
	@ln -sf "$(CURDIR)/target/debug/noslop" "$(HOME)/.cargo/bin/noslop"
	@echo "✓ Symlinked ~/.cargo/bin/noslop -> $(CURDIR)/target/debug/noslop"
	@echo ""
	@echo "Global 'noslop' now points to your development build."
	@echo "Run 'make dev' to start watching for changes."

dev-teardown: ## Remove development symlink and restore original noslop
	@echo "Tearing down development environment..."
	@if [ -L "$(HOME)/.cargo/bin/noslop" ]; then \
		rm "$(HOME)/.cargo/bin/noslop"; \
		echo "✓ Removed development symlink"; \
	fi
	@if [ -e "$(HOME)/.cargo/bin/noslop.bak" ]; then \
		mv "$(HOME)/.cargo/bin/noslop.bak" "$(HOME)/.cargo/bin/noslop"; \
		echo "✓ Restored original noslop from backup"; \
	else \
		echo "No backup found. Run 'make install' to reinstall noslop."; \
	fi

dev: dev-setup ## Start development mode with hot reload (global noslop = this repo)
	@command -v cargo-watch >/dev/null 2>&1 || { echo "Installing cargo-watch..."; cargo install cargo-watch; }
	@echo ""
	@echo "Starting hot reload... (Ctrl+C to stop)"
	@echo "The global 'noslop' command now uses your development build."
	@echo ""
	@cargo watch -x build

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
