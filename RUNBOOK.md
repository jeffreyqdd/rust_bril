# Rust Bril Runbook

## Installation

1. Install bril utilities found [here](https://github.com/sampsyo/bril).

   - `turnt` - a tool useful for running tests during compiler development
   - `brilirs` - a blazingly fast interpreter written in rust with support for floating-point operation and bit cast between char, int, float (double precision IEEE-754)
   - `brili2json`- converts from text representation to the JSON representation

2. Build using `cargo build --release`

## Instructions

Should pass the `--help` flag for more information. A couple points work highlighting:

### General Flags

- `-f|--file <FILE>` specifies the source code filepath (`rust_bril` will read from stdin otherwise)
- `-o|--out <FILE>` specifies the output code filepath

### Lesson 2 Flags

- `--transform-print` will transform the bril program by adding print statements before every `jmp` and `br` instruction
- `--construct_cfg <FILE>` will construct the code-block and write the control-flow graph to the filepath (`rust_bril` will print to stdout otherwise).
