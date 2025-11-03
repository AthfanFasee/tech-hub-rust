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



# ==================================================================================== #
# DEVELOPMENT
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

# Redis command: launch a redis container
redis:
	@echo "Launching a new redis container..."
	./scripts/init_redis.sh
