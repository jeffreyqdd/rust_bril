TRANSFORM_FIXTURES := tests/fixtures/*.json
PARSE_FIXTURES := tests/parse/*.bril
DCE_LVN_FIXTURES := tests/dce_lvn/*.bril
ALL_BENCHMARKS := benchmarks/**/*.bril

test: unit-test test-transform test-parse test-local
.PHONY: test

unit-test:
	cargo test --quiet
.PHONY: test

test-transform:
	turnt --env transform $(TRANSFORM_FIXTURES) --parallel
.PHONY: test-transform

test-parse:
	turnt --env parse $(PARSE_FIXTURES) --parallel
.PHONY: test-transform

test-local:
	turnt --env dce_lvn $(DCE_LVN_FIXTURES) --parallel
.PHONY: test-transform

make comprehensive-check:
	cargo build --release

gen: 
	turnt --env parse_baseline $(PARSE_FIXTURES) --parallel --save
	turnt --env dce_lvn_baseline $(DCE_LVN_FIXTURES) --parallel --save
	turn --env bench_baseline $(ALL_BENCHMARKS)
