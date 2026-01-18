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

# Frontend checks (only if ui/ directory exists and has changes)
if [ -d "ui" ] && [ -f "ui/package.json" ]; then
    # Check if any UI files are staged
    if git diff --cached --name-only | grep -q "^ui/"; then
        echo "4. Running frontend checks..."

        # Check if npm is available
        if ! command -v npm &> /dev/null; then
            echo "Warning: npm not found. Skipping frontend checks."
        else
            cd ui

            # Install deps if node_modules doesn't exist
            if [ ! -d "node_modules" ]; then
                echo "   Installing frontend dependencies..."
                npm ci --silent
            fi

            # Lint check
            echo "   Checking frontend linting..."
            if ! npm run lint --silent; then
                echo "Error: Frontend lint failed. Run 'npm run lint:fix' in ui/ to fix."
                exit 1
            fi

            # Format check
            echo "   Checking frontend formatting..."
            if ! npm run format:check --silent; then
                echo "Error: Frontend not formatted. Run 'npm run format' in ui/ to fix."
                exit 1
            fi

            # Type check
            echo "   Checking frontend types..."
            if ! npm run check --silent 2>/dev/null; then
                echo "Error: Frontend type check failed. Run 'npm run check' in ui/ to see errors."
                exit 1
            fi

            # Tests
            echo "   Running frontend tests..."
            if ! npm run test --silent; then
                echo "Error: Frontend tests failed. Run 'npm run test' in ui/ to see errors."
                exit 1
            fi

            cd ..
            echo "✓ Frontend checks passed"
        fi
    fi
fi

echo ""
echo "✓ All pre-commit checks passed!"
exit 0
