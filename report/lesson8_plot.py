#!/usr/bin/env python3
"""
Beautiful plotting script for Lesson 8 benchmark results analysis.

This script creates comprehensive visualizations of compilation optimization performance:
1. Dynamic instruction count comparison
2. Execution time comparison
3. Speedup heatmap (using "ssa" as baseline)
4. Standard deviation comparison across compilation flags

Author: GitHub Copilot
Created: October 26, 2025
"""

import json
import subprocess
import sys
from pathlib import Path
from typing import Dict, List, Any
import warnings

warnings.filterwarnings("ignore")

# Install required packages if not available
required_packages = ["numpy", "pandas", "matplotlib", "seaborn"]
missing_packages = []

for package in required_packages:
    try:
        __import__(package)
    except ImportError:
        missing_packages.append(package)

if missing_packages:
    print(f"Installing missing packages: {', '.join(missing_packages)}")
    subprocess.check_call([sys.executable, "-m", "pip", "install"] + missing_packages)

# Now import after installation
import numpy as np
import pandas as pd
import matplotlib.pyplot as plt
import matplotlib.colors as colors
import seaborn as sns
from matplotlib.figure import Figure

# Set up the plotting style for beautiful visualizations
plt.style.use("default")
sns.set_palette("husl")
plt.rcParams.update(
    {
        "font.size": 12,
        "axes.titlesize": 14,
        "axes.labelsize": 12,
        "xtick.labelsize": 10,
        "ytick.labelsize": 10,
        "legend.fontsize": 10,
        "figure.titlesize": 16,
        "figure.figsize": (12, 8),
        "figure.dpi": 100,
        "savefig.dpi": 300,
        "savefig.bbox": "tight",
        "axes.grid": True,
        "grid.alpha": 0.3,
    }
)


def load_benchmark_data(json_path: Any) -> List[Dict[str, Any]]:
    """Load benchmark results from JSON file."""
    with open(json_path, "r") as f:
        return json.load(f)


def get_consistent_sort_order(df: pd.DataFrame) -> List[str]:
    """Get consistent sorting order based on dynamic instruction count from SSA."""
    df_clean = df.drop_duplicates(subset=["filename", "run_name"])
    instr_pivot = df_clean.pivot(index="filename", columns="run_name", values="dyn_instruction_count")
    sort_column = "ssa" if "ssa" in instr_pivot.columns else instr_pivot.columns[0]
    return instr_pivot.sort_values(by=sort_column, ascending=False).index.tolist()


def extract_benchmark_metrics(data: List[Dict[str, Any]]) -> pd.DataFrame:
    """
    Extract key metrics from benchmark data into a structured DataFrame.

    Returns a DataFrame with columns:
    - filename: benchmark file name (cleaned)
    - run_name: compilation configuration name
    - dyn_instruction_count: dynamic instruction count
    - avg_time: average execution time
    - std_dev: standard deviation of execution time
    """
    records = []

    for benchmark in data:
        filename = Path(benchmark["filename"]).stem  # Get just the filename without extension/path

        for result in benchmark["results"]:
            records.append(
                {
                    "filename": filename,
                    "run_name": result["run_name"],
                    "dyn_instruction_count": result["dyn_instr_count"],
                    "avg_time": result["avg_time"],
                    "std_dev": result["std_dev"],
                }
            )

    return pd.DataFrame(records)


def create_instruction_count_comparison(df: pd.DataFrame) -> Figure:
    """Create dynamic instruction count comparison as a heatmap grid."""
    fig, ax = plt.subplots(figsize=(12, 20))  # Further increased height for better visibility

    # Remove duplicates by taking the first occurrence of each filename+run_name combination
    df_clean = df.drop_duplicates(subset=["filename", "run_name"])

    # Pivot data for easier plotting
    pivot_df = df_clean.pivot(index="filename", columns="run_name", values="dyn_instruction_count")

    # Ensure consistent column order
    run_order = ["original", "ssa", "loop", "lvn & dce", "all"]
    pivot_df = pivot_df.reindex(columns=run_order)

    # Use consistent sorting order
    sort_order = get_consistent_sort_order(df)
    pivot_df = pivot_df.reindex(sort_order)

    # Create heatmap with log scale
    sns.heatmap(
        pivot_df,
        annot=True,
        fmt=".0f",
        cmap="viridis",
        norm=colors.LogNorm(),  # Log scale for colors
        cbar_kws={"label": "Dynamic Instruction Count (log scale)"},
        ax=ax,
    )

    ax.set_title("Dynamic Instruction Count Comparison Across Compilation Flags\n(Sorted by instruction count)")
    ax.set_xlabel("Compilation Configuration")
    ax.set_ylabel("Benchmark Programs")

    # Rotate y-axis labels for better readability
    ax.set_yticklabels(ax.get_yticklabels(), rotation=0)

    plt.tight_layout()
    return fig


def create_execution_time_comparison(df: pd.DataFrame) -> Figure:
    """Create execution time comparison as a heatmap grid."""
    fig, ax = plt.subplots(figsize=(12, 20))  # Further increased height for better visibility

    # Remove duplicates by taking the first occurrence of each filename+run_name combination
    df_clean = df.drop_duplicates(subset=["filename", "run_name"])

    # Pivot data for easier plotting
    pivot_df = df_clean.pivot(index="filename", columns="run_name", values="avg_time")

    # Ensure consistent column order
    run_order = ["original", "ssa", "loop", "lvn & dce", "all"]
    pivot_df = pivot_df.reindex(columns=run_order)

    # Use consistent sorting order
    sort_order = get_consistent_sort_order(df)
    pivot_df = pivot_df.reindex(sort_order)

    # Convert to milliseconds for better readability
    pivot_df = pivot_df * 1000

    # Create heatmap with log scale
    sns.heatmap(
        pivot_df,
        annot=True,
        fmt=".2f",
        cmap="plasma",
        norm=colors.LogNorm(),  # Log scale for colors
        cbar_kws={"label": "Average Execution Time (ms, log scale)"},
        ax=ax,
    )

    ax.set_title("Execution Time Comparison Across Compilation Flags\n(Sorted by instruction count)")
    ax.set_xlabel("Compilation Configuration")
    ax.set_ylabel("Benchmark Programs")

    # Rotate y-axis labels for better readability
    ax.set_yticklabels(ax.get_yticklabels(), rotation=0)

    plt.tight_layout()
    return fig


def create_speedup_heatmap(df: pd.DataFrame) -> Figure:
    """Create speedup heatmap using 'ssa' as baseline (1x multiple)."""
    fig, ax = plt.subplots(figsize=(12, 20))  # Further increased height for better visibility

    # Remove duplicates by taking the first occurrence of each filename+run_name combination
    df_clean = df.drop_duplicates(subset=["filename", "run_name"])

    # Pivot data for easier manipulation
    pivot_df = df_clean.pivot(index="filename", columns="run_name", values="avg_time")

    # Ensure consistent column order
    run_order = ["original", "ssa", "loop", "lvn & dce", "all"]
    pivot_df = pivot_df.reindex(columns=run_order)

    # Use consistent sorting order
    sort_order = get_consistent_sort_order(df)
    pivot_df = pivot_df.reindex(sort_order)

    # Calculate speedup relative to 'ssa' (baseline = 1x)
    if "ssa" in pivot_df.columns:
        speedup_df = pivot_df.div(pivot_df["ssa"], axis=0)
    else:
        print("Warning: 'ssa' column not found, using first column as baseline")
        speedup_df = pivot_df.div(pivot_df.iloc[:, 0], axis=0)

    # Create heatmap
    mask = speedup_df.isna()
    sns.heatmap(
        speedup_df,
        annot=True,
        fmt=".2f",
        cmap="RdYlGn_r",  # Red for slower, Green for faster
        center=1.0,  # Center colormap at 1x (no speedup/slowdown)
        cbar_kws={"label": "Execution Time Multiple (relative to SSA)"},
        mask=mask,
        ax=ax,
    )

    ax.set_title(
        "Execution Time Speedup Heatmap\n(Lower values = faster execution, SSA = 1.0x baseline, sorted by instruction count)"
    )
    ax.set_xlabel("Compilation Configuration")
    ax.set_ylabel("Benchmark Programs")

    # Rotate y-axis labels for better readability
    ax.set_yticklabels(ax.get_yticklabels(), rotation=0)

    plt.tight_layout()
    return fig


def create_stddev_comparison(df: pd.DataFrame) -> Figure:
    """Create standard deviation comparison as a bar plot of averages for each optimization type."""
    fig, ax = plt.subplots(figsize=(10, 6))

    # Remove duplicates by taking the first occurrence of each filename+run_name combination
    df_clean = df.drop_duplicates(subset=["filename", "run_name"])

    # Calculate average standard deviation for each optimization type
    avg_stddev = df_clean.groupby("run_name")["std_dev"].mean()

    # Ensure consistent column order
    run_order = ["original", "ssa", "loop", "lvn & dce", "all"]
    avg_stddev = avg_stddev.reindex(run_order)

    # Convert to milliseconds for better readability
    avg_stddev = avg_stddev * 1000

    # Create bar plot
    colors = sns.color_palette("husl", len(run_order))
    bars = ax.bar(run_order, avg_stddev, color=colors, alpha=0.8, edgecolor='black', linewidth=0.5)

    # Add value labels on top of bars
    for bar, value in zip(bars, avg_stddev):
        if not pd.isna(value):
            ax.text(bar.get_x() + bar.get_width()/2, bar.get_height() + 0.001, 
                   f'{value:.3f}', ha='center', va='bottom', fontweight='bold')

    ax.set_title("Average Execution Time Variability by Optimization Type")
    ax.set_xlabel("Compilation Configuration")
    ax.set_ylabel("Average Standard Deviation (ms)")
    ax.grid(True, alpha=0.3, axis='y')

    plt.tight_layout()
    return fig


def create_summary_statistics_table(df: pd.DataFrame) -> pd.DataFrame:
    """Create a summary statistics table for analysis."""

    # Calculate summary statistics for each run configuration
    summary_stats = (
        df.groupby("run_name")
        .agg(
            {
                "dyn_instruction_count": ["mean", "std", "min", "max"],
                "avg_time": ["mean", "std", "min", "max"],
                "std_dev": ["mean", "std"],
            }
        )
        .round(6)
    )

    # Flatten column names
    summary_stats.columns = [f"{col[0]}_{col[1]}" for col in summary_stats.columns]

    return summary_stats


def main():
    """Main function to generate all plots and analysis."""
    print("🎨 Creating beautiful benchmark analysis plots...")

    # Load benchmark data
    data_path = Path("report/lesson8_benchmark_results.json")
    if not data_path.exists():
        print(f"❌ Error: {data_path} not found!")
        return

    print("📊 Loading benchmark data...")
    raw_data = load_benchmark_data(data_path)
    df = extract_benchmark_metrics(raw_data)

    print(
        f"✅ Loaded data for {len(df['filename'].unique())} benchmarks with {len(df['run_name'].unique())} configurations"
    )
    print(f"Run configurations: {sorted(df['run_name'].unique())}")

    # Create output directory
    output_dir = Path("report/plots")
    output_dir.mkdir(exist_ok=True)

    # Generate plots
    print("📈 Generating dynamic instruction count comparison...")
    fig1 = create_instruction_count_comparison(df)
    fig1.savefig(output_dir / "1_instruction_count_comparison.png")
    plt.close(fig1)

    print("⏱️  Generating execution time comparison...")
    fig2 = create_execution_time_comparison(df)
    fig2.savefig(output_dir / "2_execution_time_comparison.png")
    plt.close(fig2)

    print("🔥 Generating speedup heatmap...")
    fig3 = create_speedup_heatmap(df)
    fig3.savefig(output_dir / "3_speedup_heatmap.png")
    plt.close(fig3)

    print("📊 Generating standard deviation comparison...")
    fig4 = create_stddev_comparison(df)
    fig4.savefig(output_dir / "4_stddev_comparison.png")
    plt.close(fig4)

    # Generate summary statistics
    print("📋 Generating summary statistics...")
    summary_stats = create_summary_statistics_table(df)
    summary_stats.to_csv(output_dir / "summary_statistics.csv")

    # Print summary to console
    print("\n" + "=" * 80)
    print("📈 BENCHMARK ANALYSIS SUMMARY")
    print("=" * 80)

    print(f"\n🎯 Analysis covers {len(df['filename'].unique())} benchmark programs:")
    for filename in sorted(df["filename"].unique()):
        print(f"   • {filename}")

    print(f"\n🔧 Compilation configurations analyzed:")
    run_order = ["original", "ssa", "loop", "lvn & dce", "all"]
    for run_name in run_order:
        if run_name in df["run_name"].unique():
            count = len(df[df["run_name"] == run_name])
            print(f"   • {run_name}: {count} benchmarks")

    print(f"\n📊 Average metrics across all benchmarks:")
    avg_metrics = df.groupby("run_name")[["dyn_instruction_count", "avg_time", "std_dev"]].mean()
    for run_name in run_order:
        if run_name in avg_metrics.index:
            instr = float(avg_metrics.loc[run_name, "dyn_instruction_count"])
            time_ms = float(avg_metrics.loc[run_name, "avg_time"]) * 1000
            std_ms = float(avg_metrics.loc[run_name, "std_dev"]) * 1000
            print(f"   • {run_name:12}: {instr:8.0f} instructions, {time_ms:6.2f}ms ±{std_ms:5.2f}ms")

    # Calculate speedups relative to SSA
    if "ssa" in avg_metrics.index:
        ssa_time = float(avg_metrics.loc["ssa", "avg_time"])
        print(f"\n🚀 Speedup relative to SSA baseline:")
        for run_name in run_order:
            if run_name in avg_metrics.index and run_name != "ssa":
                run_time = float(avg_metrics.loc[run_name, "avg_time"])
                speedup = ssa_time / run_time
                print(f"   • {run_name:12}: {speedup:.2f}x")

    print(f"\n✅ All plots saved to: {output_dir.absolute()}")
    print(f"📁 Files generated:")
    print(f"   • 1_instruction_count_comparison.png")
    print(f"   • 2_execution_time_comparison.png")
    print(f"   • 3_speedup_heatmap.png")
    print(f"   • 4_stddev_comparison.png")
    print(f"   • summary_statistics.csv")
    print("\n🎉 Analysis complete!")


if __name__ == "__main__":
    main()
