# Rust Bril

A compiler written in Rust for for [BRIL](https://github.com/sampsyo/bril) (Big Red Intermediate Language). BRIL is a compiler IR whose canonical representation is JSON, making it extremely easy to parse.

## Features

- [x] code block and control flow graph generation
- [ ] dead code elimination

## How to use Turnt

turn --save simple.bril
turn simple.bril

# Runbook

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

### Lesson 2 Flags

- `--transform-print <FILE>` will transform the bril program by adding print statements before every `jmp` and `br` instruction (`rust_bril` will print to stdout if no file is provided).
- `--construct_cfg <FILE>` will construct the code-block and write the control-flow graph to the filepath (`rust_bril` will print to stdout if no file is provided).
