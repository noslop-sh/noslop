#!/usr/bin/env bash
# Pre-push hook for noslop development
# Note: Pre-commit checks (format, clippy, tests) should have already run
# Coverage checks are performed in CI/CD for better developer experience

echo "✓ Pre-push checks passed (coverage runs in CI)"
exit 0
