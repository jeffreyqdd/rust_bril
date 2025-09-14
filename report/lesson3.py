#!/usr/bin/env python3

from pathlib import Path
import subprocess
import json

BENCHMARKS_DIR = Path("benchmarks")
subprocess.run(["cargo", "build", "--release"], check=True)


def transform_file(f: Path) -> tuple[str, str]:
    """returns [original_condensed_program, compiler_condensed_program]"""
    print(f"getting condensed files for {file.as_posix()}")
    with open(f, "r", encoding="utf-8") as infile:

        def bril2json(infile) -> str:
            return subprocess.run(
                ["bril2json"], stdin=infile, capture_output=True, text=True, check=True
            )

        def bril2txt(stdout) -> str:
            return subprocess.run(
                ["bril2txt"], input=stdout, capture_output=True, text=True, check=True
            )

        def rust_bril(f) -> str:
            return subprocess.run(
                ["./target/release/rust_bril", "-f", f, "--local"],
                capture_output=True,
                text=True,
                check=True,
            )

        return (
            bril2txt(bril2json(infile).stdout).stdout,
            bril2txt(rust_bril(file.as_posix()).stdout).stdout,
        )


# transform each file into json and back using bril2json and bril2txt
benchmarks = {}

for file in BENCHMARKS_DIR.glob("**/*.bril"):
    original, compiled = transform_file(file)

    with open(file.with_suffix(".baseline_prof"), "r", encoding="utf-8") as infile:
        baseline_prof = int(infile.read().split(" ")[-1])

    with open(file.with_suffix(".prof"), "r", encoding="utf-8") as infile:
        prof = int(infile.read().split(" ")[-1])

    benchmarks[file.name] = {
        "original_lines": original.count("\n"),
        "original_dyn_inst": baseline_prof,
        "compiled_lines": compiled.count("\n"),
        "compiled_dyn_inst": prof,
    }

json.dump(benchmarks, open("lesson3_benchmarks.json", "w", encoding="utf-8"), indent=4)
