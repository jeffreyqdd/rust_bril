import json
import numpy as np
import matplotlib.pyplot as plt

with open("./report/lesson6_benchmarks.json", "r", encoding="utf-8") as infile:
    data = json.load(infile)

programs = list(data.keys())

original_lines = np.array([data[p]["original_lines"] for p in programs])
compiled_lines = np.array([data[p]["compiled_lines"] for p in programs])
dce_lines = np.array([data[p]["dce_lines"] for p in programs])
lvn_dce_lines = np.array([data[p]["lvn_dce_lines"] for p in programs])

original_dyn = np.array([data[p]["original_dyn_inst"] for p in programs])
compiled_dyn = np.array([data[p]["compiled_dyn_inst"] for p in programs])
dce_dyn = np.array([data[p]["dce_dyn_inst"] for p in programs])
lvn_dce_dyn = np.array([data[p]["lvn_dce_dyn_inst"] for p in programs])


# # --- Statistics ---
# reductions_static = {}
# reductions_dynamic = {}

# for prog, stats in data.items():
#     if stats["original_lines"] > 0:
#         reductions_static[prog] = (
#             (stats["original_lines"] - stats["compiled_lines"])
#             / stats["original_lines"]
#             * 100
#         )
#     if stats["original_dyn_inst"] > 0:
#         reductions_dynamic[prog] = (
#             (stats["original_dyn_inst"] - stats["compiled_dyn_inst"])
#             / stats["original_dyn_inst"]
#             * 100
#         )

# # Average reductions
# avg_static_reduction = sum(reductions_static.values()) / len(reductions_static)
# avg_dynamic_reduction = sum(reductions_dynamic.values()) / len(reductions_dynamic)

# # Greatest reductions
# greatest_static = max(reductions_static, key=reductions_static.get)
# greatest_dynamic = max(reductions_dynamic, key=reductions_dynamic.get)

# print(f"Average Static Reduction: {avg_static_reduction:.2f}%")
# print(f"Average Dynamic Reduction: {avg_dynamic_reduction:.2f}%")
# print(
#     f"Greatest Static Reduction: {greatest_static} ({reductions_static[greatest_static]:.2f}%)"
# )
# print(
#     f"Greatest Dynamic Reduction: {greatest_dynamic} ({reductions_dynamic[greatest_dynamic]:.2f}%)"
# )


# --- Plot ---
x = np.arange(len(programs))
bar_width = 0.20

fig, axes = plt.subplots(2, 1, figsize=(16, 6))

step = 5  # show every 5th program name

# --- Static line count ---
axes[0].bar(x - bar_width / 2, original_lines, bar_width, label="Original")
axes[0].bar(x + bar_width / 2, compiled_lines, bar_width, label="SSA Round Trip")
axes[0].bar(x + 3 * bar_width / 2, dce_lines, bar_width, label="SSA + DCE")
axes[0].bar(x + 5 * bar_width / 2, lvn_dce_lines, bar_width, label="SSA + LVN + DCE")
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

# --- Show into SSA instruction count
axes[1].bar(x - bar_width / 2, original_dyn, bar_width, label="Original")
axes[1].bar(x + bar_width / 2, compiled_dyn, bar_width, label="SSA Round Trip")
axes[1].bar(x + 3 * bar_width / 2, dce_dyn, bar_width, label="SSA + DCE")
axes[1].bar(x + 5 * bar_width / 2, lvn_dce_dyn, bar_width, label="SSA + LVN + DCE")

axes[1].set_xticks(x[::step])  # only every 5th tick
axes[1].set_xticklabels(
    [programs[i] for i in range(0, len(programs), step)],
    rotation=45,
    ha="right",
    fontsize=8,
)
axes[1].set_ylabel("Dynamic Instruction Count")
axes[1].set_title("Dynamic Instruction Count")
axes[1].set_yscale("log")
axes[1].legend()

plt.tight_layout()
plt.show()
