# Run command: performs check, format, and run in sequence
run:
	@echo "Running cargo check..."
	cargo check
	@echo "Running cargo fmt..."
	cargo fmt
	@echo "Running cargo run..."
	cargo run

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