# 📊 Benchmark Visualization Analysis

Beautiful plotting script for analyzing compilation optimization performance results from Lesson 8.

## 🎯 Overview

This script analyzes the performance impact of different compilation flags across 119 benchmark programs, creating comprehensive visualizations of:

1. **Dynamic Instruction Count** - Code efficiency comparison
2. **Execution Time** - Real-world performance analysis  
3. **Speedup Heatmap** - Performance relative to SSA baseline
4. **Variability Analysis** - Execution consistency metrics

## 🚀 Quick Start

```bash
# Run the plotting script
cd /Users/jeffreyqian/Projects/Cornell/cs6120/rust_bril
python3 report/lesson8_plot.py
```

The script will:
- ✅ Auto-install missing dependencies (numpy, pandas, matplotlib, seaborn)
- 📊 Load benchmark data from `lesson8_benchmark_results.json`
- 🎨 Generate beautiful visualizations in `report/plots/`
- 📈 Create summary statistics and analysis

## 📁 Generated Files

After running, you'll find in `report/plots/`:

- `1_instruction_count_comparison.png` - Dynamic instruction count bars
- `2_execution_time_comparison.png` - Average execution time comparison
- `3_speedup_heatmap.png` - Speedup heatmap (SSA = 1.0x baseline)
- `4_stddev_comparison.png` - Execution time variability analysis
- `summary_statistics.csv` - Detailed numerical statistics
- `index.html` - Beautiful HTML report with all visualizations

## 🔧 Compilation Configurations Analyzed

| Configuration | Flags | Description |
|---------------|-------|-------------|
| **original** | `["-s"]` | Original code with statistics |
| **ssa** | `[]` | SSA form only (baseline) |
| **loop** | `["--loops"]` | Loop optimizations |
| **lvn & dce** | `["--lvn", "--dce"]` | Local value numbering + dead code elimination |
| **all** | `["--lvn", "--dce", "--loops"]` | All optimizations combined |

## 📈 Key Findings

**Performance Results (relative to SSA baseline):**
- 🥇 **original**: 1.08x speedup (best performance)
- 🥈 **all**: 1.05x speedup (combined optimizations)  
- 🥉 **lvn & dce**: 1.04x speedup
- **loop**: 1.02x speedup
- **ssa**: 1.00x (baseline)

**Average Metrics:**
- **Benchmarks analyzed**: 119 programs
- **Best average time**: 8.41ms (original configuration)
- **Instruction count range**: 8-138M instructions
- **Execution time range**: 1.87ms-376ms

## 🎨 Visualization Details

### 1. Dynamic Instruction Count Comparison
- **Purpose**: Compare code efficiency across configurations
- **Interpretation**: Lower bars = more efficient compiled code
- **Key insight**: Shows how optimizations reduce instruction count

### 2. Execution Time Analysis
- **Purpose**: Real-world performance comparison
- **Units**: Milliseconds (converted from seconds)
- **Key insight**: Reveals actual runtime performance impact

### 3. Speedup Heatmap
- **Purpose**: Relative performance visualization
- **Baseline**: SSA configuration (1.0x multiplier)
- **Color coding**: 🟢 Green = faster, 🔴 Red = slower
- **Key insight**: Per-benchmark performance patterns

### 4. Variability Analysis
- **Purpose**: Execution consistency measurement
- **Metric**: Standard deviation of execution times
- **Key insight**: Lower variance = more predictable performance

## 🛠️ Technical Implementation

The script uses several advanced features:

- **Automatic dependency installation** - No manual setup required
- **Duplicate handling** - Robust data cleaning for pivot operations
- **Consistent ordering** - Same column order across all visualizations
- **Beautiful styling** - Professional matplotlib/seaborn aesthetics
- **Comprehensive analysis** - Both visual and statistical summaries

## 📊 Data Structure

The script expects JSON data with this structure:
```json
[
  {
    "filename": "benchmarks/core/example.bril",
    "results": [
      {
        "run_name": "ssa",
        "flags": [],
        "dyn_instr_count": 839,
        "avg_time": 0.002259,
        "std_dev": 0.000180
      }
    ]
  }
]
```

## 🎯 Usage Tips

1. **View HTML Report**: Open `report/plots/index.html` in a browser for the best viewing experience
2. **High-res Images**: All plots saved at 300 DPI for publication quality
3. **CSV Analysis**: Use `summary_statistics.csv` for further numerical analysis
4. **Customization**: Modify color schemes and styling in the script's matplotlib config

## 🔍 Interpretation Guide

**When analyzing results:**
- Look for consistent patterns across multiple benchmarks
- Consider both instruction count and execution time together
- Pay attention to variability - high variance may indicate unstable optimizations
- Use the heatmap to identify benchmarks that benefit most from specific optimizations

## 📝 Requirements

- Python 3.7+
- Auto-installed: numpy, pandas, matplotlib, seaborn
- Input: `lesson8_benchmark_results.json`

## 🎨 Features

✨ **Beautiful Visualizations**
- Professional color schemes
- Clear legends and labels
- Consistent formatting
- High-resolution output

📊 **Comprehensive Analysis**
- Multiple visualization types
- Statistical summaries  
- HTML report generation
- CSV data export

🛡️ **Robust Data Handling**
- Automatic duplicate removal
- Missing data handling
- Type safety
- Error reporting

---

*Generated by lesson8_plot.py - Beautiful benchmark analysis for CS 6120 compiler optimization research*