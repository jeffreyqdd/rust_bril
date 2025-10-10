#!/usr/bin/env bash
set -euo pipefail

RUST_EXECUTABLE=$1
RUST_FLAGS="--log-level error -S"

for file in $(find ./benchmarks -name '*.bril'); do
    echo "Running test on $file"
    $RUST_EXECUTABLE $RUST_FLAGS $file | python3 tests/ssa/is_ssa.py
done