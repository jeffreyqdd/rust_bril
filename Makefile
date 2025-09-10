TRANSFORM_FIXTURES := tests/fixtures/*.json
PARSE_FIXTURES := tests/parse/*.bril
DCE_FIXTURES := tests/dce/*.bril

test: unit-test test-transform test-parse test-dce
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

test-dce:
	turnt --env dce $(DCE_FIXTURES) --parallel
.PHONY: test-transform

gen: 
	turnt --env parse_baseline $(PARSE_FIXTURES) --parallel --save
	turnt --env dce_baseline $(DCE_FIXTURES) --parallel --save

