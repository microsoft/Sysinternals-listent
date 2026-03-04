#!/bin/bash

# Test runner script that runs all tests and provides a total summary
# Usage: ./tests/run_all_tests.sh [cargo test options]

set -o pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo -e "${BLUE}=== Running All Tests ===${NC}"
echo ""

# Create a temp file for test output
TEMP_OUTPUT=$(mktemp)

# Run cargo test and capture output
cargo test "$@" 2>&1 | tee "$TEMP_OUTPUT"
TEST_EXIT_CODE=${PIPESTATUS[0]}

echo ""
echo -e "${BLUE}========================================${NC}"
echo -e "${BLUE}         TEST SUMMARY                   ${NC}"
echo -e "${BLUE}========================================${NC}"

# Parse the output to get totals
# Each test target outputs a line like: "test result: ok. X passed; Y failed; Z ignored"
TOTAL_PASSED=0
TOTAL_FAILED=0
TOTAL_IGNORED=0

while IFS= read -r line; do
    if [[ "$line" =~ ^test\ result:.*([0-9]+)\ passed.*([0-9]+)\ failed.*([0-9]+)\ ignored ]]; then
        # Extract numbers using grep
        passed=$(echo "$line" | grep -oE '[0-9]+ passed' | grep -oE '[0-9]+')
        failed=$(echo "$line" | grep -oE '[0-9]+ failed' | grep -oE '[0-9]+')
        ignored=$(echo "$line" | grep -oE '[0-9]+ ignored' | grep -oE '[0-9]+')

        if [[ -n "$passed" ]]; then
            TOTAL_PASSED=$((TOTAL_PASSED + passed))
        fi
        if [[ -n "$failed" ]]; then
            TOTAL_FAILED=$((TOTAL_FAILED + failed))
        fi
        if [[ -n "$ignored" ]]; then
            TOTAL_IGNORED=$((TOTAL_IGNORED + ignored))
        fi
    fi
done < "$TEMP_OUTPUT"

# Calculate total
TOTAL_TESTS=$((TOTAL_PASSED + TOTAL_FAILED + TOTAL_IGNORED))

# Print summary with colors
echo ""
if [[ $TOTAL_FAILED -eq 0 ]]; then
    echo -e "${GREEN}✓ TOTAL PASSED:  $TOTAL_PASSED${NC}"
else
    echo -e "${GREEN}  TOTAL PASSED:  $TOTAL_PASSED${NC}"
fi

if [[ $TOTAL_FAILED -gt 0 ]]; then
    echo -e "${RED}✗ TOTAL FAILED:  $TOTAL_FAILED${NC}"
else
    echo -e "${RED}  TOTAL FAILED:  $TOTAL_FAILED${NC}"
fi

if [[ $TOTAL_IGNORED -gt 0 ]]; then
    echo -e "${YELLOW}⊘ TOTAL SKIPPED: $TOTAL_IGNORED${NC}"
else
    echo -e "${YELLOW}  TOTAL SKIPPED: $TOTAL_IGNORED${NC}"
fi

echo -e "${BLUE}  ─────────────────────${NC}"
echo -e "${BLUE}  TOTAL TESTS:   $TOTAL_TESTS${NC}"
echo ""

# Cleanup
rm -f "$TEMP_OUTPUT"

# Exit with the original test exit code
if [[ $TOTAL_FAILED -gt 0 ]]; then
    echo -e "${RED}Some tests failed!${NC}"
    exit 1
else
    echo -e "${GREEN}All tests passed!${NC}"
    exit 0
fi
