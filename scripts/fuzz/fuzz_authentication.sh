#!/bin/bash
set -e

echo "=========================================="
echo "Fuzzing Authentication Domain"
echo "=========================================="

FUZZERS=(
    "fuzz_user_email"
    "fuzz_user_password"
    "fuzz_user_email_unicode"
    "fuzz_password_grapheme_counting"
    "fuzz_login_json"
    "fuzz_register_json"
)

DURATION=${DURATION:-60}
MAX_LEN=512

# Use nightly if Makefile passed: CARGO="/usr/bin/env cargo +nightly"
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
echo "Authentication fuzzing complete!"
echo "=========================================="
