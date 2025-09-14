import json
import numpy as np
import matplotlib.pyplot as plt

with open("./report/lesson3_benchmarks.json", "r", encoding="utf-8") as infile:
    data = json.load(infile)

programs = list(data.keys())
original_lines = [data[p]["original_lines"] for p in programs]
compiled_lines = [data[p]["compiled_lines"] for p in programs]
original_dyn = [data[p]["original_dyn_inst"] for p in programs]
compiled_dyn = [data[p]["compiled_dyn_inst"] for p in programs]

# --- Statistics ---
reductions_static = {}
reductions_dynamic = {}

for prog, stats in data.items():
    if stats["original_lines"] > 0:
        reductions_static[prog] = (
            (stats["original_lines"] - stats["compiled_lines"])
            / stats["original_lines"]
            * 100
        )
    if stats["original_dyn_inst"] > 0:
        reductions_dynamic[prog] = (
            (stats["original_dyn_inst"] - stats["compiled_dyn_inst"])
            / stats["original_dyn_inst"]
            * 100
        )

# Average reductions
avg_static_reduction = sum(reductions_static.values()) / len(reductions_static)
avg_dynamic_reduction = sum(reductions_dynamic.values()) / len(reductions_dynamic)

# Greatest reductions
greatest_static = max(reductions_static, key=reductions_static.get)
greatest_dynamic = max(reductions_dynamic, key=reductions_dynamic.get)

print(f"Average Static Reduction: {avg_static_reduction:.2f}%")
print(f"Average Dynamic Reduction: {avg_dynamic_reduction:.2f}%")
print(
    f"Greatest Static Reduction: {greatest_static} ({reductions_static[greatest_static]:.2f}%)"
)
print(
    f"Greatest Dynamic Reduction: {greatest_dynamic} ({reductions_dynamic[greatest_dynamic]:.2f}%)"
)


# --- Plot ---
x = np.arange(len(programs))
bar_width = 0.35

fig, axes = plt.subplots(2, 1, figsize=(16, 6))

step = 5  # show every 5th program name

# --- Static line count ---
axes[0].bar(x - bar_width / 2, original_lines, bar_width, label="Original Lines")
axes[0].bar(x + bar_width / 2, compiled_lines, bar_width, label="Compiled Lines")
axes[0].set_xticks(x[::step])  # only every 5th tick
axes[0].set_xticklabels(
    [programs[i] for i in range(0, len(programs), step)],
    rotation=45,
    ha="right",
    fontsize=8,
)
axes[0].set_ylabel("Line Count")
axes[0].set_title("Static Line Count")
axes[0].legend()

# --- Dynamic instruction count ---
axes[1].bar(x - bar_width / 2, original_dyn, bar_width, label="Original Dyn Inst")
axes[1].bar(x + bar_width / 2, compiled_dyn, bar_width, label="Compiled Dyn Inst")
axes[1].set_xticks(x[::step])  # only every 5th tick
axes[1].set_xticklabels(
    [programs[i] for i in range(0, len(programs), step)],
    rotation=45,
    ha="right",
    fontsize=8,
)
axes[1].set_ylabel("Dynamic Instruction Count (log scale)")
axes[1].set_title("Dynamic Instruction Count")
axes[1].set_yscale("log")
axes[1].legend()

plt.tight_layout()
plt.show()
