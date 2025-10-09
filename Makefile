# Run command: performs check, format, and run in sequence
run:
	@echo "Running cargo fmt..."
	cargo fmt
	@echo "Running cargo check..."
	cargo check
	@echo "Running cargo run with with bunyan formatted logs..."
	cargo run | bunyan


# Test command: performs check, format, and test
test:
	@echo "Running cargo check..."
	cargo check
	@echo "Running cargo fmt..."
	cargo fmt
	@echo "Running cargo test"
	@unset RUST_LOG && unset TEST_LOG && cargo test

# Test log command: performs check, format, then runs tests with bunyan logging
test-log:
	@echo "Running cargo check..."
	cargo check
	@echo "Running cargo fmt..."
	cargo fmt
	@echo "Running cargo test with bunyan formatted logs..."
	@export RUST_LOG="sqlx=error,info,error" && export TEST_LOG=true && cargo test | bunyan

# Test log debug command: performs check, format, then runs tests with bunyan logging
test-log-debug:
	@echo "Running cargo check..."
	cargo check
	@echo "Running cargo fmt..."
	cargo fmt
	@echo "Running cargo test with bunyan formatted logs in debug mode..."
	@export RUST_LOG="sqlx=error,debug" && export TEST_LOG=true && cargo test | bunyan

# Test log debug command: performs check, format, then runs tests with bunyan logging
# Usage: make test-single name=your_test_name
test-single:
	@if [ -z "$(name)" ]; then \
		echo "Error: please provide a test name, e.g.:"; \
		echo "   make test-single name=logout_clears_session_state"; \
		exit 1; \
	fi
	@echo "Running cargo check..."
	cargo check
	@echo "Running cargo fmt..."
	cargo fmt
	@echo "Running cargo test for '$(name)' with bunyan formatted logs in debug mode..."
	@export RUST_LOG="sqlx=error,debug" && export TEST_LOG=true && cargo test $(name) -- --nocapture | bunyan

# Create new migration file
migrate-add:
	@if [ -z "$(name)" ]; then \
		echo "Error: please provide a migration file name, e.g.:"; \
		echo "   make migrate-add name=create_tokens_table"; \
		exit 1; \
	fi
	sqlx migrate add $(name)

# Migrate command: performs sql query caching for sqlx, creates the db in docker container, migrates the db
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


# Redis command: launch a redis container
redis:
	@echo "Launching a new redis container..."
	./scripts/init_redis.sh
