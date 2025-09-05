
JSON_FIXTURES := tests/fixtures/*.json


test:
	cargo test
.PHONY: test

test-transform:
	turnt --env transform $(JSON_FIXTURES)
.PHONY: test-transform

