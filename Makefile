# ==================================================================================== #
# PHONY DECLARATIONS
# ==================================================================================== #

.PHONY: \
	lint security-audit audit security-audit-full audit-full \
	run run-full run-scratch \
	test test-full test-log test-log-debug test-single test-release \
	migrate-add migrate migrate-new \
	fuzz fuzz-single fuzz-domain fuzz-intensive fuzz-coverage fuzz-clean \
	redis



# ==================================================================================== #
# QUALITY CONTROL
# ==================================================================================== #

# Lint command: runs cargo update, formatting and checking
lint:
	@echo "Updating dependencies to latest compatible versions..."
	@cargo update
	@echo "Running cargo fmt..."
	cargo fmt
	@echo "Running cargo check..."
	cargo check

# Security audit command: runs cargo update, cargo audit and cargo deny in sequence for checking vulnerabilities
security-audit:
	@echo "Updating dependencies to latest compatible versions..."
	@cargo update
	@echo "Running security audit for vulnerabilities..."
	@cargo audit || (echo "CRITICAL: Security vulnerabilities found!")
	@echo "Checking for banned crates (ignoring duplicates)..."
	@cargo deny check bans -A duplicate || (echo "CRITICAL: Banned crates found!")
	@echo "Verifying dependency sources..."
	@cargo deny check sources || (echo "CRITICAL: Untrusted dependency sources detected!")
	@echo "All security checks passed!"

# Full audit command: runs security audit and tests
audit: security-audit test
	@echo "Full audit completed successfully!"

# Security audit full command: runs security audit, tests, and fuzz
security-audit-full: security-audit test
	@echo ""
	@echo "=========================================="
	@echo "Running full security audit with fuzzing"
	@echo "Estimated time: ~15 minutes"
	@echo "=========================================="
	@$(MAKE) fuzz
	@echo ""
	@echo "=========================================="
	@echo "Full security audit completed successfully!"
	@echo "=========================================="

# Full audit with fuzz: runs security audit, tests, and fuzz
audit-full: security-audit-full
	@echo "Full audit with fuzzing completed successfully!"



# ==================================================================================== #
# RUN
# ==================================================================================== #

# Run command: performs check, format, and run in sequence
run: lint
	@echo "Running cargo run with with bunyan formatted logs..."
	cargo run | bunyan

# Run full command: starts existing containers and runs the app
run-full: lint
	@echo "Starting existing Postgres and Redis containers..."
	@docker start techhub_postgres || echo "Postgres container not found. Run 'make run-scratch' instead."
	@docker start techhub_redis || echo "Redis container not found. Run 'make run-scratch' instead."
	@echo "Running cargo run with bunyan formatted logs..."
	cargo run | bunyan

# Run from scratch: creates new containers, runs migrations, and starts the app
run-scratch: migrate-new redis
	@echo "Running cargo run with bunyan formatted logs..."
	cargo run | bunyan



# ==================================================================================== #
# TEST
# ==================================================================================== #

# Test command: performs check, format, and test (skips doc-tests)
test: lint
	@echo "Running cargo test (skipping doc-tests)"
	@unset RUST_LOG && unset TEST_LOG && cargo test --all-targets

# Test full command: starts existing containers and runs tests
test-full: lint
	@echo "Starting existing Postgres and Redis containers..."
	@docker start techhub_postgres || echo "Postgres container not found. Run 'make run-scratch' instead."
	@docker start techhub_redis || echo "Redis container not found. Run 'make run-scratch' instead."
	@$(MAKE) test

# Test log command: performs check, format, then runs tests with bunyan logging
test-log: lint
	@echo "Running cargo test with bunyan formatted logs..."
	@export RUST_LOG="sqlx=error,info,error" && export TEST_LOG=true && cargo test --all-targets | bunyan

# Test log debug command: performs check, format, then runs tests with bunyan logging
test-log-debug: lint
	@echo "Running cargo test with bunyan formatted logs in debug mode..."
	@export RUST_LOG="debug" && export TEST_LOG=true && cargo test --all-targets | bunyan

# Test single command: performs check, format, then runs tests with bunyan logging
# Usage: make test-single name=your_test_name
test-single: lint
	@if [ -z "$(name)" ]; then \
		echo "Error: please provide a test name, e.g.:"; \
		echo "   make test-single name=logout_clears_session_state"; \
		exit 1; \
	fi
	@echo "Running cargo test for '$(name)' with bunyan formatted logs in debug mode..."
	@export RUST_LOG="sqlx=error,debug" && export TEST_LOG=true && cargo test --all-targets $(name) -- --nocapture | bunyan

# Test release command: runs tests in release mode
test-release: lint
	@echo "Running cargo test in release mode (skipping doc-tests)"
	@unset RUST_LOG && unset TEST_LOG && cargo test --release --all-targets
	@echo "Release mode test completed!"

# Test stress command: runs only unit tests multiple times for property-based testing
unit-test-stress-quick: lint
	@echo "Running unit tests 10 times for quick property-based testing..."
	@for i in $$(seq 1 10); do \
		echo "=== Unit test run $$i/10 ==="; \
		unset RUST_LOG && unset TEST_LOG && cargo test --lib || exit 1; \
	done
	@echo "Quick stress test completed! All 10 runs passed."

unit-test-stress-standard: lint
	@echo "Running unit tests 20 times for thorough property-based testing..."
	@for i in $$(seq 1 20); do \
		echo "=== Unit test run $$i/20 ==="; \
		unset RUST_LOG && unset TEST_LOG && cargo test --lib || exit 1; \
	done
	@echo "Standard stress test completed! All 20 runs passed."

unit-test-stress-thorough: lint
	@echo "Running unit tests 50 times for exhaustive property-based testing..."
	@for i in $$(seq 1 50); do \
		echo "=== Unit test run $$i/50 ==="; \
		unset RUST_LOG && unset TEST_LOG && cargo test --lib || exit 1; \
	done
	@echo "Thorough stress test completed! All 50 runs passed."

# Test stress with logging command: runs unit tests 10 times with bunyan logging
unit-test-stress-log: lint
	@echo "Running unit tests 10 times with bunyan logging..."
	@for i in $$(seq 1 10); do \
		echo "=== Unit test run $$i/10 ==="; \
		export RUST_LOG="sqlx=error,info,error" && export TEST_LOG=true && cargo test --lib | bunyan || exit 1; \
	done
	@echo "Unit stress test with logging completed! All 10 runs passed."



# ==================================================================================== #
# MIGRATION
# ==================================================================================== #

# Create new migration file
migrate-add:
	@if [ -z "$(name)" ]; then \
		echo "Error: please provide a migration file name, e.g.:"; \
		echo "   make migrate-add name=create_tokens_table"; \
		exit 1; \
	fi
	sqlx migrate add $(name)

# Migrate command: performs sql query caching for sqlx, creates the db in docker container if required, migrates the db
migrate:
	@echo "Running db migrations (skip docker)..."
	SKIP_DOCKER=true ./scripts/init_db.sh
	@echo "Running cargo sqlx prepare..."
	cargo sqlx prepare

# Migrate new command: delete existing container and re-init db
migrate-new:
	@echo "Stopping and removing existing postgres container..."
	-docker rm -f techhub_postgres || true
	@echo "Re-initializing db container and running migrations..."
	./scripts/init_db.sh
	@echo "Running cargo sqlx prepare..."
	cargo sqlx prepare



# ==================================================================================== #
# FUZZING
# ==================================================================================== #

# Ensure nightly exists (silent if already installed)
ensure-nightly:
	@rustup toolchain list | grep nightly > /dev/null || rustup toolchain install nightly

# Initialize corpus: runs the corpus setup script (this ensures the corpus files are created)
# Usage: make init-fuzz-corpus
init-fuzz-corpus:
	@echo "=========================================="
	@echo "Initializing fuzz corpus..."
	@echo "=========================================="
	@./scripts/fuzz/setup_fuzz_corpus.sh

# Fuzz command: runs all fuzz domains with nightly
fuzz: ensure-nightly init-fuzz-corpus
	@echo "=========================================="
	@echo "WARNING: Long-running operation"
	@echo "Estimated time: ~10 minutes"
	@echo "=========================================="
	@echo "Running all fuzzing domains using nightly toolchain..."
	@CARGO='/usr/bin/env cargo +nightly' ./scripts/fuzz/fuzz_authentication.sh
	@CARGO='/usr/bin/env cargo +nightly' ./scripts/fuzz/fuzz_posts.sh
	@CARGO='/usr/bin/env cargo +nightly' ./scripts/fuzz/fuzz_comments.sh
	@CARGO='/usr/bin/env cargo +nightly' ./scripts/fuzz/fuzz_newsletter.sh
	@echo ""
	@echo "=========================================="
	@echo "All fuzzing completed successfully!"
	@echo "=========================================="

# Fuzz single command: runs a specific fuzzer target
# Usage: make fuzz-single name=fuzz_user_email duration=60
fuzz-single: ensure-nightly init-fuzz-corpus
	@if [ -z "$(name)" ]; then \
		echo "Error: please provide a fuzzer name, e.g.:"; \
		echo "   make fuzz-single name=fuzz_user_email"; \
		echo "   make fuzz-single name=fuzz_user_email duration=120"; \
		exit 1; \
	fi
	@duration=${duration:-60}; \
	echo "Running fuzzer '$(name)' for ${duration}s..."; \
	cargo +nightly fuzz run $(name) -- -max_len=512 -max_total_time=${duration} || echo "Fuzzer found issues (check fuzz/artifacts/)"

# Fuzz domain command: runs all fuzzers for a specific domain
# Usage: make fuzz-domain name=authentication
fuzz-domain: ensure-nightly init-fuzz-corpus
	@if [ -z "$(name)" ]; then \
		echo "Error: please provide a domain name, e.g.:"; \
		echo "   make fuzz-domain name=authentication"; \
		exit 1; \
	fi
	@if [ ! -f "./scripts/fuzz_$(name).sh" ]; then \
		echo "Error: Script './scripts/fuzz_$(name).sh' not found"; \
		exit 1; \
	fi
	@echo "Running fuzzing for $(name) domain..."
	@CARGO='/usr/bin/env cargo +nightly' ./scripts/fuzz_$(name).sh

# Fuzz intensive command: runs all fuzzers with extended duration (5 minutes each)
fuzz-intensive: ensure-nightly init-fuzz-corpus
	@echo "=========================================="
	@echo "WARNING: Very long-running operation"
	@echo "Estimated time: ~40 minutes"
	@echo "=========================================="
	@echo "Running intensive fuzzing (300s per fuzzer)..."
	@DURATION=300 CARGO='/usr/bin/env cargo +nightly' ./scripts/fuzz/fuzz_authentication.sh
	@DURATION=300 CARGO='/usr/bin/env cargo +nightly' ./scripts/fuzz/fuzz_posts.sh
	@DURATION=300 CARGO='/usr/bin/env cargo +nightly' ./scripts/fuzz/fuzz_comments.sh
	@DURATION=300 CARGO='/usr/bin/env cargo +nightly' ./scripts/fuzz/fuzz_newsletter.sh
	@echo ""
	@echo "=========================================="
	@echo "Intensive fuzzing completed!"
	@echo "=========================================="

# Fuzz coverage command: generates coverage report for a fuzzer
# Usage: make fuzz-coverage name=fuzz_user_email
fuzz-coverage: ensure-nightly
	@if [ -z "$(name)" ]; then \
		echo "Error: please provide a fuzzer name, e.g.:"; \
		echo "   make fuzz-coverage name=fuzz_user_email"; \
		exit 1; \
	fi
	@echo "Generating coverage report for '$(name)'..."
	cargo +nightly fuzz coverage $(name)
	@echo "Opening coverage report..."
	@open fuzz/coverage/$(name)/index.html || xdg-open fuzz/coverage/$(name)/index.html || echo "Coverage report at: fuzz/coverage/$(name)/index.html"

# Fuzz clean command: removes fuzz artifacts and corpus
fuzz-clean: ensure-nightly
	@echo "Cleaning fuzzing artifacts and corpus..."
	@rm -rf fuzz/artifacts/*
	@rm -rf fuzz/corpus/*
	@echo "Fuzzing data cleaned!"



# ==================================================================================== #
# REDIS
# ==================================================================================== #

# Redis command: launch a redis container
redis:
	@echo "Launching a new redis container..."
	./scripts/init_redis.sh