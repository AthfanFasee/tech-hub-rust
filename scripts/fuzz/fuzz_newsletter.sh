#!/bin/bash
set -e

echo "=========================================="
echo "Fuzzing Newsletter Domain"
echo "=========================================="

FUZZERS=(
    "fuzz_newsletter_json"
)

DURATION=${DURATION:-60}
MAX_LEN=512

CARGO_CMD="${CARGO:-cargo}"

for fuzzer in "${FUZZERS[@]}"; do
    echo ""
    echo "Running $fuzzer (${DURATION}s)..."
    $CARGO_CMD fuzz run "$fuzzer" -- \
        -max_len="$MAX_LEN" \
        -max_total_time="$DURATION" \
        || echo "$fuzzer found issues (check artifacts/)"
done

echo ""
echo "=========================================="
echo "Newsletter fuzzing complete!"
echo "=========================================="