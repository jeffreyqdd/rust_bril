import time
import json
import statistics
import subprocess
import multiprocessing

from tqdm import tqdm
from pathlib import Path
from dataclasses import dataclass

BENCHMARKS_DIR = Path("benchmarks")

compilation_flags = {
    "original": ["-s"],
    "ssa": [],
    "loop": ["--loops"],
    "lvn & dce": ["--lvn", "--dce"],
    "all": ["--lvn", "--dce", "--loops"],
}


@dataclass(frozen=True)
class BrilBenchmarkInstance:
    filename: str
    arguments: list[str]
    compiled_code: list[str]
    compiled_flags: list[tuple[str, list[str]]]


def generate_benchmark_instance(f: Path) -> BrilBenchmarkInstance:
    """Generate benchmark data for a single file"""

    # Extract arguments from the file
    arg_extraction = subprocess.run(
        [
            "awk",
            "/^# ARGS:/ {for (i=3; i<=NF; i++) print $i}",
            f.as_posix(),
        ],
        capture_output=True,
        text=True,
        check=True,
    )

    raw_arguments = arg_extraction.stdout.strip().replace("\n", " ")
    arguments = raw_arguments.split(" ") if raw_arguments else []

    compiled_code = []
    compiled_flags = []
    for name, flags in compilation_flags.items():
        compile_process = subprocess.run(
            ["./target/release/rust_bril", "--log-level=off"] + flags + [f.as_posix()],
            capture_output=True,
            text=True,
            check=True,
        )

        compiled_code.append(compile_process.stdout)
        compiled_flags.append((name, flags))

    return BrilBenchmarkInstance(
        filename=f.as_posix(),
        arguments=arguments,
        compiled_code=compiled_code,
        compiled_flags=compiled_flags,
    )


def run_benchmark(bm: BrilBenchmarkInstance, runs: int = 500):
    """Run benchmark for a given BrilBenchmarkInstance using hyperfine with stability improvements"""
    import tempfile
    import os
    import psutil

    try:
        current_process = psutil.Process()
        current_process.nice(-19)
    except (psutil.AccessDenied, OSError):
        # Permission denied or not supported, continue anyway
        pass

    results = []

    for i, (code, flags) in enumerate(zip(bm.compiled_code, bm.compiled_flags)):
        # Create temporary file for the compiled code
        with tempfile.NamedTemporaryFile(mode="w", suffix=".bril", delete=False) as temp_file:
            temp_file.write(code)
            temp_filename = temp_file.name

        # Create temporary JSON file for hyperfine output
        with tempfile.NamedTemporaryFile(mode="w", suffix=".json", delete=False) as json_file:
            json_filename = json_file.name

        # Build hyperfine command
        flag_name = flags[0]

        # Write JSON code directly to temp file (code is already in JSON format from rust_bril)
        with tempfile.NamedTemporaryFile(mode="w", suffix=".json", delete=False) as json_temp_file:
            json_temp_file.write(code)
            json_temp_filename = json_temp_file.name

        # Build the command - brilirs reads JSON
        if bm.arguments:
            command = f"brilirs {' '.join(bm.arguments)} < {json_temp_filename}"
        else:
            command = f"brilirs < {json_temp_filename}"

        # Build the command as a single string for shell=True
        dyn_inst_command = f"brilirs -p {' '.join(bm.arguments)} < {json_temp_filename}"
        dyn_inst_proc = subprocess.run(
            dyn_inst_command,
            shell=True,
            capture_output=True,
            text=True,
            check=True,
        )
        dyn_inst_output = dyn_inst_proc.stderr.strip()

        # Enhanced hyperfine command with stability options
        hyperfine_cmd = [
            "hyperfine",
            "--show-output",
            "--warmup",
            "10",  # 10 warmup runs to stabilize performance
            "--min-runs",
            "20",  # Minimum 20 runs even if variance is low
            "--max-runs",
            str(runs * 2),  # Allow up to 2x runs if needed for stability
            "--export-json",
            json_filename,
            "--command-name",
            flag_name,
            command,
        ]

        try:
            # Run hyperfine - let's see what the actual error is first
            result_proc = subprocess.run(
                hyperfine_cmd, check=True, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL
            )

        except subprocess.CalledProcessError as e:
            # If hyperfine fails, let's see what went wrong
            print(f"Hyperfine failed for {flag_name}: {e}")
            print(f"Command: {' '.join(hyperfine_cmd)}")
            if e.stderr:
                print(f"Error output: {e.stderr}")
            # Skip this configuration and continue
            continue

        try:  # Read and parse hyperfine results
            with open(json_filename, "r") as f:
                hyperfine_data = json.load(f)

            # Extract the benchmark result (hyperfine returns array with one result)
            bench_result = hyperfine_data["results"][0]

            # Statistical validation and outlier filtering
            raw_times = bench_result.get("times", [])

            # Calculate coefficient of variation for stability assessment
            cv = bench_result["stddev"] / bench_result["mean"] if bench_result["mean"] > 0 else float("inf")

            # Convert hyperfine data to our format with stability metrics
            result = {
                "run_name": flag_name,
                "flags": flags[1],
                "runs": runs,
                "dyn_instr_count": int(dyn_inst_output.strip().split(":")[-1]),
                "avg_time": bench_result["mean"],
                "std_dev": bench_result["stddev"],
                "min_time": bench_result["min"],
                "max_time": bench_result["max"],
                "median_time": bench_result["median"],
                "coefficient_of_variation": cv,
                "stability_rating": "good" if cv < 0.05 else "fair" if cv < 0.15 else "poor",
                "times": raw_times,
            }

            # Report stability
            if cv > 0.15:
                print(f"    ⚠️  High variance for {flag_name}: CV={cv:.3f}")

            results.append(result)

        finally:
            # Clean up temporary files
            os.unlink(temp_filename)
            os.unlink(json_temp_filename)
            os.unlink(json_filename)

    return {
        "filename": bm.filename,
        "results": results,
    }


def check_system_stability():
    """Check system conditions for stable benchmarking"""
    import subprocess
    import time

    print("Checking system stability...")

    # Check CPU frequency scaling (macOS)
    try:
        result = subprocess.run(["sysctl", "-n", "hw.cpufrequency_max"], capture_output=True, text=True, check=False)
        if result.returncode == 0:
            print(f"   Max CPU frequency: {int(result.stdout.strip()) / 1e9:.2f} GHz")
    except:
        pass

    # Check thermal state (macOS)
    try:
        result = subprocess.run(["pmset", "-g", "therm"], capture_output=True, text=True, check=False)
        if "CPU_Speed_Limit" in result.stdout:
            print("   CPU thermal throttling detected!")
        else:
            print("   CPU thermal state: Normal")
    except:
        pass


def main():
    """Main function to run benchmarks using multiprocessing"""

    # Check system stability before starting
    check_system_stability()

    # Collect all .bril files
    bril_files = list(BENCHMARKS_DIR.glob("**/*.bril"))
    cores = 4  # Reduced for stability

    print(f"\n🚀 Found {len(bril_files)} .bril files to process")
    print(f"Using {cores} processes for parallel execution (reduced for benchmark stability)")

    with multiprocessing.Pool(processes=cores) as pool:
        result = list(
            tqdm(pool.imap(generate_benchmark_instance, bril_files), total=len(bril_files), desc="Processing files")
        )
    with multiprocessing.Pool(processes=cores) as pool2:
        benchmarks = list(tqdm(pool2.imap(run_benchmark, result), total=len(result), desc="Running benchmarks"))

    # write benchmarks to json file
    with open("./report/lesson8_benchmark_results.json", "w", encoding="utf-8") as outfile:
        json.dump(benchmarks, outfile, indent=4)


if __name__ == "__main__":
    subprocess.run(["cargo", "build", "--release"], check=True)
    main()
