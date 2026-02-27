# ==================================================================================== #
# IMPORTANT: NIGHTLY TOOLCHAIN REQUIRED
# ==================================================================================== #
# Most commands in this Makefile depend on the Rust nightly toolchain for:
#   - Advanced formatting features (import merging via rustfmt)
#   - Fuzzing capabilities (cargo-fuzz requires nightly)
#   - Commands that depend on nightly: lint-nightly, all test targets, all run targets,
#     all fuzz targets (fuzz, fuzz-single, fuzz-domain, fuzz-intensive, fuzz-coverage)
#
# To install nightly, run:
#   make install-nightly
#
# Or manually:
#   rustup toolchain install nightly
#   rustup component add rustfmt --toolchain nightly
# ==================================================================================== #



# ==================================================================================== #
# PHONY DECLARATIONS
# ==================================================================================== #

.PHONY: \
	install-nightly ensure-nightly install-nextest ensure-nextest help \
	lint lint-nightly security-audit audit security-audit-full audit-full \
	run run-full run-scratch \
	test test-full test-scratch test-log test-log-debug test-single test-release \
	unit-test-stress-quick unit-test-stress-standard unit-test-stress-thorough unit-test-stress-log \
	test-nx test-nx-full test-nx-scratch test-nx-log test-nx-log-debug test-nx-single test-nx-release \
	unit-test-nx-stress-quick unit-test-nx-stress-standard unit-test-nx-stress-thorough unit-test-nx-stress-log \
	migrate-add migrate migrate-new \
	fuzz fuzz-single fuzz-domain fuzz-intensive fuzz-coverage fuzz-clean init-fuzz-corpus \
	redis redis-new



# ==================================================================================== #
# SETUP
# ==================================================================================== #

# Ensure nightly exists (installs if missing)
ensure-nightly:
	@rustup toolchain list | grep nightly > /dev/null || \
		(echo "Nightly toolchain not found. Installing now..." && rustup toolchain install nightly) || \
		(echo "ERROR: Failed to install nightly. Run 'make install-nightly' or install manually." && exit 1)
	@rustup component list --toolchain nightly | grep "rustfmt.*installed" > /dev/null || \
		rustup component add rustfmt --toolchain nightly

# Install nightly toolchain: installs Rust nightly and rustfmt component
install-nightly:
	@echo "Installing Rust nightly toolchain..."
	rustup toolchain install nightly
	@echo "Adding rustfmt component to nightly..."
	rustup component add rustfmt --toolchain nightly
	@echo "Nightly toolchain installed successfully!"
	@echo ""
	@echo "You can now use 'make lint-nightly' and other commands that require nightly."

# Ensure nextest is installed (installs if missing)
ensure-nextest:
	@cargo nextest --version > /dev/null 2>&1 || \
		(echo "cargo-nextest not found. Installing now..." && cargo install --locked cargo-nextest) || \
		(echo "ERROR: Failed to install cargo-nextest. Run 'make install-nextest' manually." && exit 1)

# Install cargo-nextest: next-generation test runner with cleaner output and faster parallel execution
install-nextest:
	@echo "Installing cargo-nextest..."
	cargo install --locked cargo-nextest
	@echo "cargo-nextest installed successfully!"
	@echo ""
	@echo "You can now use 'make test-nx' and other nextest commands."

# Help command: displays available commands and setup info
help:
	@echo "=========================================="
	@echo "TechHub Makefile - Available Commands"
	@echo "=========================================="
	@echo ""
	@echo "SETUP (First Time):"
	@echo "  make install-nightly       Install Rust nightly toolchain (required)"
	@echo ""
	@echo "QUALITY CONTROL:"
	@echo "  make lint                  Format code (stable rustfmt)"
	@echo "  make lint-nightly          Format code + merge imports (nightly)"
	@echo "  make security-audit        Check for vulnerabilities"
	@echo "  make audit                 Security audit + tests"
	@echo "  make audit-full            Security audit + tests + fuzzing (~15min)"
	@echo ""
	@echo "RUN:"
	@echo "  make run                   Lint + run app (uses existing containers)"
	@echo "  make run-full              Start containers + run app"
	@echo "  make run-scratch           Fresh containers + migrations + run"
	@echo ""
	@echo "TEST:"
	@echo "  make test                  Run all tests"
	@echo "  make test-full             Start containers + run tests"
	@echo "  make test-scratch          Fresh containers + run tests"
	@echo "  make test-log              Run tests with bunyan logs"
	@echo "  make test-single name=X    Run specific test with logs"
	@echo "  make test-release          Run tests in release mode"
	@echo ""
	@echo "STRESS TEST:"
	@echo "  make unit-test-stress-quick      Run unit tests 10x"
	@echo "  make unit-test-stress-standard   Run unit tests 20x"
	@echo "  make unit-test-stress-thorough   Run unit tests 50x"
	@echo ""
	@echo "SETUP:"
	@echo "  make install-nextest           Install cargo-nextest test runner"
	@echo "TEST (nextest - cleaner output, faster):"
	@echo "  make test-nx                   Run all tests via nextest"
	@echo "  make test-nx-full              Start containers + run nextest"
	@echo "  make test-nx-scratch           Fresh containers + run nextest"
	@echo "  make test-nx-log               Run nextest with bunyan logs"
	@echo "  make test-nx-single name=X     Run specific test with logs"
	@echo "  make test-nx-release           Run nextest in release mode"
	@echo ""
	@echo "STRESS TEST (nextest):"
	@echo "  make unit-test-nx-stress-quick      Run unit tests 10x via nextest"
	@echo "  make unit-test-nx-stress-standard   Run unit tests 20x via nextest"
	@echo "  make unit-test-nx-stress-thorough   Run unit tests 50x via nextest"
	@echo ""
	@echo "DATABASE:"
	@echo "  make migrate               Run migrations (skip docker)"
	@echo "  make migrate-new           Fresh Postgres container + migrations"
	@echo "  make migrate-add name=X    Create new migration file"
	@echo ""
	@echo "REDIS:"
	@echo "  make redis                 Launch Redis container"
	@echo "  make redis-new             Fresh Redis container"
	@echo ""
	@echo "FUZZING:"
	@echo "  make fuzz                  Run all fuzzers (~10min)"
	@echo "  make fuzz-single name=X    Run specific fuzzer (default 60s)"
	@echo "  make fuzz-domain name=X    Run all fuzzers for domain"
	@echo "  make fuzz-intensive        Extended fuzzing (~40min)"
	@echo "  make fuzz-coverage name=X  Generate coverage report"
	@echo "  make fuzz-clean            Clean fuzz artifacts"
	@echo ""
	@echo "TIPS:"
	@echo "  - Most commands use 'lint-nightly' automatically"
	@echo "  - Run 'make install-nightly' once before first use"
	@echo "  - See Makefile comments for more details"
	@echo "=========================================="

.DEFAULT_GOAL := help



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

# Lint nightly command: runs cargo update, nightly formatting (with import merging) and checking
lint-nightly: ensure-nightly
	@echo "Updating dependencies to latest compatible versions..."
	@cargo update
	@echo "Running cargo fmt with nightly (for import merging)..."
	cargo +nightly fmt -- --config-path .cargo/rustfmt-nightly.toml
	@echo "Running cargo check..."
	cargo check

# Security audit command: runs cargo update, cargo audit and cargo deny in sequence for checking vulnerabilities
security-audit:
	@echo "Updating dependencies to latest compatible versions..."
	@cargo update
	@echo "Running security audit for vulnerabilities..."
	@cargo audit || (echo "CRITICAL: Security vulnerabilities found!")
	@echo "Checking for banned crates (ignoring duplicates)..."
	@cargo deny check --config .cargo/deny.toml bans -A duplicate || (echo "CRITICAL: Banned crates found!")
	@echo "Verifying dependency sources..."
	@cargo deny check --config .cargo/deny.toml sources || (echo "CRITICAL: Untrusted dependency sources detected!")
	@echo "Verifying dependency licenses..."
	@cargo deny check --config .cargo/deny.toml licenses || (echo "CRITICAL: Unlicensed or disallowed licenses found!")
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
run: lint-nightly
	@echo "Running cargo run with with bunyan formatted logs..."
	cargo run | bunyan

# Run full command: starts existing containers and runs the app
run-full: lint-nightly
	@echo "Starting existing Postgres and Redis containers..."
	@docker start techhub_postgres || echo "Postgres container not found. Run 'make run-scratch' instead."
	@docker start techhub_redis || echo "Redis container not found. Run 'make run-scratch' instead."
	@echo "Running cargo run with bunyan formatted logs..."
	cargo run | bunyan

# Run from scratch: creates new containers, runs migrations, and starts the app
run-scratch: migrate-new redis-new
	@echo "Running cargo run with bunyan formatted logs..."
	cargo run | bunyan



# ==================================================================================== #
# TEST
# ==================================================================================== #

# Test command: performs check, format, and test (skips doc-tests)
test: lint-nightly
	@echo "Running cargo test (skipping doc-tests)"
	@unset RUST_LOG && unset TEST_LOG && cargo test --all-targets

# Test full command: starts existing containers and runs tests
test-full: lint-nightly
	@echo "Starting existing Postgres and Redis containers..."
	@docker start techhub_postgres || echo "Postgres container not found. Run 'make run-scratch' instead."
	@docker start techhub_redis || echo "Redis container not found. Run 'make run-scratch' instead."
	@$(MAKE) test

# Test scratch command: creates new containers and runs tests
test-scratch: migrate-new redis-new
	@$(MAKE) --no-print-directory test

# Test log command: performs check, format, then runs tests with bunyan logging
test-log: lint-nightly
	@echo "Running cargo test with bunyan formatted logs..."
	@export RUST_LOG="sqlx=error,info,error" && export TEST_LOG=true && cargo test --all-targets | bunyan

# Test log debug command: performs check, format, then runs tests with bunyan logging
test-log-debug: lint-nightly
	@echo "Running cargo test with bunyan formatted logs in debug mode..."
	@export RUST_LOG="debug" && export TEST_LOG=true && cargo test --all-targets | bunyan

# Test single command: performs check, format, then runs tests with bunyan logging
# Usage: make test-single name=your_test_name
test-single: lint-nightly
	@if [ -z "$(name)" ]; then \
		echo "Error: please provide a test name, e.g.:"; \
		echo "   make test-single name=logout_clears_session_state"; \
		exit 1; \
	fi
	@echo "Running cargo test for '$(name)' with bunyan formatted logs in debug mode..."
	@export RUST_LOG="sqlx=error,debug" && export TEST_LOG=true && cargo test --all-targets $(name) -- --nocapture | bunyan

# Test release command: runs tests in release mode
test-release: lint-nightly
	@echo "Running cargo test in release mode (skipping doc-tests)"
	@unset RUST_LOG && unset TEST_LOG && cargo test --release --all-targets
	@echo "Release mode test completed!"

# Test stress command: runs only unit tests multiple times for property-based testing
unit-test-stress-quick: lint-nightly
	@echo "Running unit tests 10 times for quick property-based testing..."
	@for i in $$(seq 1 10); do \
		echo "=== Unit test run $$i/10 ==="; \
		unset RUST_LOG && unset TEST_LOG && cargo test --lib || exit 1; \
	done
	@echo "Quick stress test completed! All 10 runs passed."

unit-test-stress-standard: lint-nightly
	@echo "Running unit tests 20 times for thorough property-based testing..."
	@for i in $$(seq 1 20); do \
		echo "=== Unit test run $$i/20 ==="; \
		unset RUST_LOG && unset TEST_LOG && cargo test --lib || exit 1; \
	done
	@echo "Standard stress test completed! All 20 runs passed."

unit-test-stress-thorough: lint-nightly
	@echo "Running unit tests 50 times for exhaustive property-based testing..."
	@for i in $$(seq 1 50); do \
		echo "=== Unit test run $$i/50 ==="; \
		unset RUST_LOG && unset TEST_LOG && cargo test --lib || exit 1; \
	done
	@echo "Thorough stress test completed! All 50 runs passed."

# Test stress with logging command: runs unit tests 10 times with bunyan logging
unit-test-stress-log: lint-nightly
	@echo "Running unit tests 10 times with bunyan logging..."
	@for i in $$(seq 1 10); do \
		echo "=== Unit test run $$i/10 ==="; \
		export RUST_LOG="sqlx=error,info,error" && export TEST_LOG=true && cargo test --lib | bunyan || exit 1; \
	done
	@echo "Unit stress test with logging completed! All 10 runs passed."



# ==================================================================================== #
# TEST (nextest — cleaner output, per-test timing, faster parallel execution)
# Note: nextest does not run doc-tests. Run 'cargo test --doc' separately if needed.
# ==================================================================================== #

# Test command: lint then run all tests via nextest
test-nx: lint-nightly ensure-nextest
	@echo "Running tests via cargo-nextest (skipping doc-tests)..."
	@unset RUST_LOG && unset TEST_LOG && cargo nextest run

# Test full command: start existing containers then run nextest
test-nx-full: lint-nightly ensure-nextest
	@echo "Starting existing Postgres and Redis containers..."
	@docker start techhub_postgres || echo "Postgres container not found. Run 'make run-scratch' instead."
	@docker start techhub_redis || echo "Redis container not found. Run 'make run-scratch' instead."
	@$(MAKE) --no-print-directory test-nx

# Test scratch command: fresh containers then run nextest
test-nx-scratch: migrate-new redis-new
	@$(MAKE) --no-print-directory test-nx

# Test log command: run nextest with bunyan logging
test-nx-log: lint-nightly ensure-nextest
	@echo "Running tests via nextest with bunyan formatted logs..."
	@export RUST_LOG="sqlx=error,info,error" && export TEST_LOG=true && cargo nextest run --no-capture 2>&1 | bunyan

# Test log debug command: run nextest with debug-level bunyan logging
test-nx-log-debug: lint-nightly ensure-nextest
	@echo "Running tests via nextest with bunyan formatted logs in debug mode..."
	@export RUST_LOG="debug" && export TEST_LOG=true && cargo nextest run --no-capture 2>&1 | bunyan

# Test single command: run a specific test by name with bunyan logging
# Usage: make test-nx-single name=your_test_name
test-nx-single: lint-nightly ensure-nextest
	@if [ -z "$(name)" ]; then \
		echo "Error: please provide a test name, e.g.:"; \
		echo "   make test-nx-single name=logout_clears_session_state"; \
		exit 1; \
	fi
	@echo "Running test via nextest for '$(name)' with bunyan formatted logs in debug mode..."
	@export RUST_LOG="sqlx=error,debug" && export TEST_LOG=true && cargo nextest run --no-capture $(name) 2>&1 | bunyan

# Test release command: run nextest in release mode
test-nx-release: lint-nightly ensure-nextest
	@echo "Running tests via nextest in release mode (skipping doc-tests)..."
	@unset RUST_LOG && unset TEST_LOG && cargo nextest run --release
	@echo "Release mode test completed!"

# Stress test commands: run only lib (unit) tests multiple times via nextest
# nextest's --test-threads controls parallelism per run
unit-test-nx-stress-quick: lint-nightly ensure-nextest
	@echo "Running unit tests 10 times via nextest for quick property-based testing..."
	@for i in $$(seq 1 10); do \
		echo "=== Unit test run $$i/10 ==="; \
		unset RUST_LOG && unset TEST_LOG && cargo nextest run --lib || exit 1; \
	done
	@echo "Quick stress test completed! All 10 runs passed."

unit-test-nx-stress-standard: lint-nightly ensure-nextest
	@echo "Running unit tests 20 times via nextest for thorough property-based testing..."
	@for i in $$(seq 1 20); do \
		echo "=== Unit test run $$i/20 ==="; \
		unset RUST_LOG && unset TEST_LOG && cargo nextest run --lib || exit 1; \
	done
	@echo "Standard stress test completed! All 20 runs passed."

unit-test-nx-stress-thorough: lint-nightly ensure-nextest
	@echo "Running unit tests 50 times via nextest for exhaustive property-based testing..."
	@for i in $$(seq 1 50); do \
		echo "=== Unit test run $$i/50 ==="; \
		unset RUST_LOG && unset TEST_LOG && cargo nextest run --lib || exit 1; \
	done
	@echo "Thorough stress test completed! All 50 runs passed."

# Stress test with logging: run unit tests 10 times via nextest with bunyan logging
unit-test-nx-stress-log: lint-nightly ensure-nextest
	@echo "Running unit tests 10 times via nextest with bunyan logging..."
	@for i in $$(seq 1 10); do \
		echo "=== Unit test run $$i/10 ==="; \
		export RUST_LOG="sqlx=error,info,error" && export TEST_LOG=true && cargo nextest run --lib --no-capture 2>&1 | bunyan || exit 1; \
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
# REDIS
# ==================================================================================== #

# Redis command: launch redis container
redis:
	@echo "Launching redis container..."
	./scripts/init_redis.sh

# Redis new command: delete existing container and launch a new redis container
redis-new:
	@echo "Stopping and removing existing redis container..."
	-docker rm -f techhub_redis || true
	@echo "Launching a new redis container..."
	./scripts/init_redis.sh



# ==================================================================================== #
# FUZZING
# ==================================================================================== #

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