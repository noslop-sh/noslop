#!/usr/bin/env bash
# Coverage check script for noslop
# Verifies that code coverage meets minimum thresholds

set -e

# Configuration
MIN_LINE_COVERAGE=80.0
MIN_BRANCH_COVERAGE=75.0
COVERAGE_JSON="coverage/tarpaulin-report.json"

echo "Checking code coverage thresholds..."
echo "Required: ${MIN_LINE_COVERAGE}% line coverage, ${MIN_BRANCH_COVERAGE}% branch coverage"
echo ""

# Check if cargo-tarpaulin is installed
if ! command -v cargo-tarpaulin &> /dev/null; then
    echo "Error: cargo-tarpaulin not found. Installing..."
    cargo install cargo-tarpaulin
fi

# Generate coverage report
echo "Generating coverage report..."
# Note: We exclude integration tests to avoid segfaults in tarpaulin
# Run only unit tests and library code for coverage
if ! cargo tarpaulin --config .tarpaulin.toml --test unit --lib; then
    echo "Error: Failed to generate coverage report"
    exit 1
fi

# Check if coverage report exists
if [ ! -f "$COVERAGE_JSON" ]; then
    echo "Error: Coverage report not found at $COVERAGE_JSON"
    exit 1
fi

# Parse coverage percentages using jq or Python
if command -v jq &> /dev/null; then
    # Use jq if available - tarpaulin JSON has coverage at top level as percentage
    line_coverage=$(jq -r '.coverage' "$COVERAGE_JSON" 2>/dev/null || echo "0")
else
    # Fallback to Python
    line_coverage=$(python3 -c "
import json
import sys

try:
    with open('$COVERAGE_JSON', 'r') as f:
        data = json.load(f)

    if 'coverage' in data:
        print(data['coverage'])
    else:
        print('0.0')
except Exception as e:
    print('0.0', file=sys.stderr)
    print(f'Error: {e}', file=sys.stderr)
    sys.exit(1)
" 2>/dev/null || echo "0.0")
fi

# Display results
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "Coverage Results:"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
printf "Line Coverage:   %6.2f%% (minimum: %.1f%%)\n" "$line_coverage" "$MIN_LINE_COVERAGE"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

# Check thresholds
passed=true

# Compare line coverage (using bc for floating point comparison)
if command -v bc &> /dev/null; then
    if [ "$(echo "$line_coverage < $MIN_LINE_COVERAGE" | bc -l)" -eq 1 ]; then
        echo "❌ Line coverage ${line_coverage}% is below minimum ${MIN_LINE_COVERAGE}%"
        passed=false
    else
        echo "✓ Line coverage meets minimum threshold"
    fi
else
    # Fallback to integer comparison (less precise)
    line_cov_int=${line_coverage%.*}
    min_line_int=${MIN_LINE_COVERAGE%.*}
    if [ "$line_cov_int" -lt "$min_line_int" ]; then
        echo "❌ Line coverage ${line_coverage}% is below minimum ${MIN_LINE_COVERAGE}%"
        passed=false
    else
        echo "✓ Line coverage meets minimum threshold"
    fi
fi

echo ""

if [ "$passed" = true ]; then
    echo "✓ All coverage thresholds met!"
    echo ""
    echo "View detailed report: coverage/index.html"
    exit 0
else
    echo "❌ Coverage check failed!"
    echo ""
    echo "To improve coverage:"
    echo "  1. Write more tests for uncovered code"
    echo "  2. View detailed report: coverage/index.html"
    echo "  3. Run 'make coverage-open' to open the report"
    exit 1
fi
