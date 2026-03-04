#!/bin/bash

# Test runner script for functional tests
# Usage: ./tests/run_functional_tests.sh
# Ensures binary is built and runs tests with proper setup

set -e

echo "=== Building listent for functional tests ==="
cargo build --release

echo "=== Running functional tests ==="

# Run specific test categories
echo "--- Static scan tests ---"
cargo test --test functional_static_scan -- --nocapture

echo "--- Monitor mode tests ---"
cargo test --test functional_monitor -- --nocapture

echo "--- Comprehensive integration tests ---"
cargo test --test functional_comprehensive -- --nocapture

echo "=== All functional tests completed ==="