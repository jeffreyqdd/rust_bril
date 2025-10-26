ALL_BENCHMARKS := benchmarks/**/*.bril

check:
	cargo build --release
	turnt --env check_ssa $(ALL_BENCHMARKS) --parallel --verbose
	turnt --env check_dce $(ALL_BENCHMARKS) --parallel --verbose
	turnt --env check_lvn_dce $(ALL_BENCHMARKS) --parallel --verbose
	turnt --env check_loop $(ALL_BENCHMARKS) --parallel --verbose
.PHONY: bench-check 

bench: 
	cargo build --release
	turnt --env bench_reference $(ALL_BENCHMARKS) --parallel --save
	turnt --env bench_ssa $(ALL_BENCHMARKS) --parallel --save
	turnt --env bench_ssa_dce $(ALL_BENCHMARKS) --parallel --save
	turnt --env bench_ssa_lvn_dce $(ALL_BENCHMARKS) --parallel --save
.PHONY: bench

bench-loop:
	turnt --env bench_ssa_loop $(ALL_BENCHMARKS) --parallel --save
.PHONY: bench-loop

bench-all-optimizations:
	turnt --env bench_ssa_lvn_dce_loop $(ALL_BENCHMARKS) --parallel --save
.PHONY: bench-all

gen-check:
	turnt --env check_reference $(ALL_BENCHMARKS) --parallel --save
.PHONY: gen-check

