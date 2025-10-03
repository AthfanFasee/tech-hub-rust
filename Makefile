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

# Test-log command: performs check, format, then runs tests with bunyan logging
test-log:
	@echo "Running cargo check..."
	cargo check
	@echo "Running cargo fmt..."
	cargo fmt
	@echo "Running cargo test with bunyan formatted logs..."
	@export RUST_LOG="sqlx=error,info" && export TEST_LOG=true && cargo test | bunyan

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

# Migrate new: delete existing container and re-init db
migrate-new:
	@echo "Stopping and removing existing postgres container..."
	-docker rm -f techhub_postgres || true
	@echo "Re-initializing db container and running migrations..."
	./scripts/init_db.sh
	@echo "Running cargo sqlx prepare..."
	cargo sqlx prepare
