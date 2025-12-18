RUST_MIN_STACK ?= 16777216
export RUST_MIN_STACK
RUST_LOG ?= info
export RUST_LOG

test-meter:
	@echo "All tests passed"
	cargo test --release exe_metered_cost -- --test-threads 1 --nocapture

test-meter-failure:
	@echo "::test_execute_cost_legacy would failed"
	cargo test --release --features=legacy exe_metered_cost -- --test-threads 1 --nocapture	

test-core-dump:
	@echo "::test proving cause segment fault (or some memory corruption error)"
	cargo test --features=cuda --release test_proving_core_dump -- --nocapture	

test-core-dump-cpu:
	@echo "::test proving with cpu also cause segment fault (or some memory corruption error)"
	cargo test --release test_proving_core_dump -- --nocapture

test-core-dump-execute:
	@echo "::test to verify the execution is ok"
	cargo test --features=cuda --release test_proving_execute_ok -- --nocapture

test-core-dump-passed:
	@echo "::test passed with 'legacy-v1-3'"
	cargo test --features=cuda,legacy --release test_proving_core_dump -- --nocapture