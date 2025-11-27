RUST_MIN_STACK ?= 16777216
export RUST_MIN_STACK

test:
	@echo "All tests passed"
	cargo test --release -- --test-threads 1 --nocapture

test-failure:
	@echo "::test_execute_cost_legacy would failed"
	cargo test --release --features=legacy -- --test-threads 1 --nocapture	