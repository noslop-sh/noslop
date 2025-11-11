#!/usr/bin/env bash
# Pre-commit hook for noslop
# Runs formatting checks and linting before allowing commits

set -e

echo "Running pre-commit checks..."

# Check if cargo is available
if ! command -v cargo &> /dev/null; then
    echo "Error: cargo not found. Please install Rust."
    exit 1
fi

# Format check
echo "1. Checking code formatting..."
if ! cargo fmt --all -- --check; then
    echo "Error: Code is not formatted. Run 'cargo fmt' or 'make fmt' to fix."
    exit 1
fi
echo "✓ Formatting check passed"

# Clippy check
echo "2. Running clippy..."
if ! cargo clippy --all-targets --all-features -- -D warnings; then
    echo "Error: Clippy found issues. Fix them before committing."
    exit 1
fi
echo "✓ Clippy check passed"

# Run tests
echo "3. Running tests..."
if ! cargo test --quiet; then
    echo "Error: Tests failed. Fix them before committing."
    exit 1
fi
echo "✓ Tests passed"

echo ""
echo "✓ All pre-commit checks passed!"
exit 0
