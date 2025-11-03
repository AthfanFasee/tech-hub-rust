#!/bin/bash
set -e

echo "=========================================="
echo "Fuzzing Comments Domain"
echo "=========================================="

FUZZERS=(
    "fuzz_comment_json"
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
echo "Comments fuzzing complete!"
echo "=========================================="
