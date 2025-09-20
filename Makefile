TRANSFORM_FIXTURES := tests/fixtures/*.json
PARSE_FIXTURES := tests/parse/*.bril
DCE_LVN_FIXTURES := tests/dce_lvn/*.bril
DATAFLOW_FIXTURES := tests/dataflow/*.bril
ALL_BENCHMARKS := benchmarks/**/*.bril

test:
	cargo build --release
	cargo test --quiet
	turnt --env transform $(TRANSFORM_FIXTURES) --parallel
	turnt --env parse $(PARSE_FIXTURES) --parallel
	turnt --env dce_lvn $(DCE_LVN_FIXTURES) --parallel
.PHONY: test

bench-check:
	cargo build --release
	turnt --env bench_check $(ALL_BENCHMARKS) --parallel
.PHONY: bench-check 

bench-local:
	cargo build --release
	turnt --env bench_local $(ALL_BENCHMARKS) --parallel
.PHONY: bench-local

gen-test: 
	turnt --env parse_baseline $(PARSE_FIXTURES) --parallel --save
	turnt --env dce_lvn_baseline $(DCE_LVN_FIXTURES) --parallel --save
	turnt --env dataflow_initialized_variables $(DATAFLOW_FIXTURES) --parallel --save
	turnt --env dataflow_live_variables $(DATAFLOW_FIXTURES) --parallel --save
.PHONY: gen-test

gen-bench:
	turnt --env bench_baseline $(ALL_BENCHMARKS) --parallel --save
	turnt --env bench_baseline_profile $(ALL_BENCHMARKS) --parallel --save
	turnt --env bench_local_profile $(ALL_BENCHMARKS) --parallel --save
.PHONY: gen-bench
