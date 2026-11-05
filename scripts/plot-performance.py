#!/usr/bin/env python3
"""Plot the recorded benchmark and profiler results. Requires Matplotlib."""

import argparse
import csv
from pathlib import Path

import matplotlib

matplotlib.use("Agg")

import matplotlib.pyplot as plt
from matplotlib.ticker import PercentFormatter


ROOT = Path(__file__).resolve().parent.parent
STAGES = {
    "parse_discovery": ("Discovery and parsing", "#0072B2"),
    "ir_lowering": ("IR lowering", "#D55E00"),
    "codegen": ("Code generation", "#009E73"),
    "end_to_end": ("End to end", "#7857A3"),
}
SECONDS_PER_UNIT = {"s": 1, "ms": 1e-3, "µs": 1e-6, "ns": 1e-9}


def criterion_seconds(value):
    amount, unit = value.split()
    return float(amount) * SECONDS_PER_UNIT[unit]


def read_benchmarks(path):
    """Read the paired runs from the baseline table in performance.md."""
    measurements = []
    _, heading, contents = path.read_text(encoding="utf-8").partition("### Baseline\n")
    if not heading:
        raise ValueError(f"No benchmark baseline section in {path}")
    baseline = contents.split("\n##", 1)[0]
    for line in baseline.splitlines():
        if not line.startswith("|"):
            continue
        files, stage, *metrics = [cell.strip() for cell in line.strip("|").split("|")]
        if stage not in STAGES:
            continue
        paired_runs = (metric.split(" / ") for metric in metrics)
        for run, values in enumerate(zip(*paired_runs, strict=True), 1):
            criterion, duration, heap, calls = values
            measurements.append({
                "files": int(files.replace(",", "")),
                "stage": stage,
                "run": run,
                "criterion": None if criterion == "Incomplete" else criterion_seconds(criterion),
                "duration": int(duration) / 1e9,
                "heap": int(heap) / 2**20,
                "calls": int(calls),
            })
    if not measurements:
        raise ValueError(f"No recorded benchmark rows in {path}")
    return measurements


def plot_benchmarks(measurements, panels, title, output, *, sharey=False):
    fig, axes = plt.subplots(1, len(panels), figsize=(12, 5.4), sharex=True, sharey=sharey)
    sizes = sorted({row["files"] for row in measurements})
    for ax, (metric, label) in zip(axes, panels, strict=True):
        for stage, (name, color) in STAGES.items():
            for run, style in ((1, "-"), (2, "--")):
                rows = sorted(
                    (row["files"], row[metric])
                    for row in measurements
                    if row["stage"] == stage and row["run"] == run
                )
                if rows:
                    files, values = zip(*rows)
                    ax.plot(files, values, style, marker="o", color=color, label=name if run == 1 else None)
        ax.set(xscale="log", yscale="log", ylabel=label, xlabel="TypeScript files")
        ax.set_xticks(sizes, labels=[f"{size:,}" for size in sizes])
        ax.grid(axis="y", alpha=0.25)

    fig.suptitle(title, x=0.06, ha="left", weight="bold", fontsize=17)
    fig.legend(*axes[0].get_legend_handles_labels(), loc="upper center", bbox_to_anchor=(0.5, 0.9), ncol=4, frameon=False)
    fig.text(0.06, 0.053, "Seed 630063 · d21a00a · solid = run 1 · dashed = run 2 · gaps indicate missing observations")
    fig.text(0.06, 0.026, "The 10,240-file tier has no recorded Criterion or one-shot benchmark measurements.")
    fig.tight_layout(rect=(0.03, 0.11, 0.98, 0.82), w_pad=3)
    fig.savefig(output, dpi=160)
    plt.close(fig)


def plot_profiles(directory, output):
    cpu = "Thread CPU delta (µs)"
    cumulative = "Cumulative bytes"
    peak = "Live bytes at global heap peak"
    cpu_captures = ("large-cpu-repeat.tsv", "huge-cpu-120s-prefix.tsv")
    heap_captures = ("large-heap.tsv", "huge-heap-ir-prefix.tsv")
    groups = (
        ("Provenance index", "#D55E00", {"IR provenance index"}),
        ("ESTree JSON conversion", "#0072B2", {
            "ESTree JSON conversion (caller unspecified)",
            "ESTree JSON conversion in discovery",
            "ESTree JSON conversion in source loading",
        }),
    )
    panels = (
        ("Recorded thread CPU", cpu, cpu_captures, "120 s prefix"),
        ("Cumulative allocation bytes", cumulative, heap_captures, "360 s prefix"),
        ("Live bytes at observed peak", peak, heap_captures, "360 s prefix"),
    )

    fig, axes = plt.subplots(3, 1, figsize=(10, 8.2), sharex=True)
    for ax, (title, metric, captures, scope) in zip(axes, panels, strict=True):
        for index, capture in enumerate(captures):
            path = directory / capture
            with path.open(encoding="utf-8", newline="") as stream:
                for line in stream:
                    if line.startswith("Category\t"):
                        rows = list(csv.DictReader([line, *stream], delimiter="\t"))
                        break
                else:
                    raise ValueError(f"No profile summary header in {path}")
            values = {row["Category"]: int(row[metric]) for row in rows if row["Category"] != "Total"}
            total = sum(values.values())
            left = 0
            for name, color, categories in groups:
                share = sum(values.get(category, 0) for category in categories) / total
                ax.barh(index, share, left=left, color=color, label=name if index == 0 else None)
                left += share
            ax.barh(index, 1 - left, left=left, color="#C4CDD6", label="All other categories" if index == 0 else None)
        ax.set_title(title, loc="left", weight="bold")
        ax.set_yticks([0, 1], ["1,000 files · complete", f"10,240 files · {scope}"])
        ax.invert_yaxis()
        ax.grid(axis="x", alpha=0.2)
    axes[-1].set(xlim=(0, 1), xlabel="Share within each capture")
    axes[-1].xaxis.set_major_formatter(PercentFormatter(xmax=1))
    fig.suptitle("Where the generator spent time and memory", x=0.06, ha="left", weight="bold", fontsize=17)
    fig.legend(*axes[0].get_legend_handles_labels(), loc="upper center", bbox_to_anchor=(0.5, 0.94), ncol=3, frameon=False)
    fig.text(0.06, 0.02, "Seed 630063 · c1e5273 · massive prefixes cover parsing and early IR.")
    fig.tight_layout(rect=(0.03, 0.05, 0.98, 0.89), h_pad=2)
    fig.savefig(output, dpi=160)
    plt.close(fig)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output-dir", type=Path, default=ROOT / "target" / "performance-charts")
    parser.add_argument("--benchmark-only", action="store_true", help="skip the issue #64 profiler chart")
    args = parser.parse_args()

    measurements = read_benchmarks(ROOT / "docs" / "performance.md")
    args.output_dir.mkdir(parents=True, exist_ok=True)
    plot_benchmarks(
        measurements,
        (("criterion", "Criterion wall time (s)"), ("duration", "Single-pass wall time (s)")),
        "Generator wall time by project size",
        args.output_dir / "benchmark-wall-time.png",
        sharey=True,
    )
    plot_benchmarks(
        measurements,
        (("heap", "Peak extra heap (MiB)"), ("calls", "Allocation calls")),
        "Generator allocations by project size",
        args.output_dir / "benchmark-allocations.png",
    )
    if not args.benchmark_only:
        plot_profiles(ROOT / "docs" / "profiles" / "issue-64", args.output_dir / "profile-hot-spots.png")
    print(args.output_dir.resolve())


if __name__ == "__main__":
    main()
