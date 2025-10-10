from pathlib import Path
import subprocess
import json

BENCHMARKS_DIR = Path("benchmarks")
subprocess.run(["cargo", "build", "--release"], check=True)


def transform_file(f: Path) -> tuple[str, str, str, str]:
    """returns [original_condensed_program, dce]"""
    print(f"getting condensed files for {file.as_posix()}")
    with open(f, "r", encoding="utf-8") as infile:

        def bril2json(infile) -> subprocess.CompletedProcess[str]:
            return subprocess.run(
                ["bril2json"], stdin=infile, capture_output=True, text=True, check=True
            )

        def bril2txt(stdout) -> subprocess.CompletedProcess[str]:
            return subprocess.run(
                ["bril2txt"], input=stdout, capture_output=True, text=True, check=True
            )

        def rust_bril(f) -> subprocess.CompletedProcess[str]:
            return subprocess.run(
                ["./target/release/rust_bril", f],
                capture_output=True,
                text=True,
                check=True,
            )

        def rust_bril_dce(f) -> subprocess.CompletedProcess[str]:
            return subprocess.run(
                ["./target/release/rust_bril", f, "--dce"],
                capture_output=True,
                text=True,
                check=True,
            )

        def rust_bril_lvn_dce(f) -> subprocess.CompletedProcess[str]:
            return subprocess.run(
                ["./target/release/rust_bril", f, "--lvn", "--dce"],
                capture_output=True,
                text=True,
                check=True,
            )

        return (
            bril2txt(bril2json(infile).stdout).stdout,
            bril2txt(rust_bril(file.as_posix()).stdout).stdout,
            bril2txt(rust_bril_dce(file.as_posix()).stdout).stdout,
            bril2txt(rust_bril_lvn_dce(file.as_posix()).stdout).stdout,
        )


# transform each file into json and back using bril2json and bril2txt
benchmarks = {}

for file in BENCHMARKS_DIR.glob("**/*.bril"):
    original, ssa, ssa_dce, ssa_lvn_dce = transform_file(file)

    with open(file.with_suffix(".baseline_prof"), "r", encoding="utf-8") as infile:
        baseline_prof = int(infile.read().split(" ")[-1])

    with open(file.with_suffix(".ssa_prof"), "r", encoding="utf-8") as infile:
        ssa_prof = int(infile.read().split(" ")[-1])

    with open(file.with_suffix(".ssa_dce_prof"), "r", encoding="utf-8") as infile:
        ssa_dce_prof = int(infile.read().split(" ")[-1])

    with open(file.with_suffix(".ssa_lvn_dce_prof"), "r", encoding="utf-8") as infile:
        ssa_lvn_dce_prof = int(infile.read().split(" ")[-1])

    benchmarks[file.name] = {
        "original_lines": original.count("\n"),
        "original_dyn_inst": baseline_prof,
        "compiled_lines": ssa.count("\n"),
        "compiled_dyn_inst": ssa_prof,
        "dce_lines": ssa_dce.count("\n"),
        "dce_dyn_inst": ssa_dce_prof,
        "lvn_dce_lines": ssa_lvn_dce.count("\n"),
        "lvn_dce_dyn_inst": ssa_lvn_dce_prof,
    }

json.dump(
    benchmarks,
    open("./report/lesson6_benchmarks.json", "w", encoding="utf-8"),
    indent=4,
)
