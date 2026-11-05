#!/usr/bin/env python3
"""Summarize the main-thread Samply capture or DHAT heap from a profile run."""

import argparse
from collections import Counter, defaultdict
import gzip
import json
from pathlib import Path


def group(frames):
    stack = "\n".join(frames)
    if "index_node_files" in stack:
        return "IR provenance index"
    if "ValidatorSource>::new" in stack and "BTreeMap<usize, &str>" in stack:
        return "IR provenance index"
    if any(name in stack for name in ("lower_project", "parse_function_ast", "parse_schema_ast")):
        return "Other IR lowering"
    if "serde_json::value::Value as core::clone::Clone" in stack:
        return "JSON AST clone"
    if any(name in stack for name in ("serde_json::de::", "ValueVisitor", "ESTree", "to_estree_json")):
        if "expand_relative_import_paths" in stack or "convex_typegen::fs::rcfp" in stack:
            return "ESTree JSON conversion in discovery"
        if "create_function_asts" in stack:
            return "ESTree JSON conversion in source loading"
        return "ESTree JSON conversion (caller unspecified)"
    if "codegen::" in stack:
        return "Rust codegen"
    if "write_atomic" in stack:
        return "Atomic output write"
    if any(
        name in stack
        for name in ("std::fs::", "std::sys::fs::", "std::sys::pal::unix::fs::", "__getattrlist")
    ):
        return "Source filesystem"
    if any(name in stack for name in ("oxc_parser::", "oxc_semantic::", "oxc_allocator::")):
        return "Oxc parse, semantics and arenas"
    if any(
        name in stack
        for name in ("parse_project", "create_function_asts", "generate_javascript_ast", "convex_typegen::fs::")
    ):
        return "Other parse and discovery"
    return "Unattributed or outside pipeline"


def cpu_summary(directory):
    with gzip.open(directory / "cpu.json.gz", "rt") as file:
        profile = json.load(file)
    sidecar = json.loads((directory / "cpu.json.syms.json").read_text())
    symbols = {
        (lib["debug_name"], lib["code_id"]): {
            address: sidecar["string_table"][lib["symbol_table"][index]["symbol"]]
            for address, index in lib["known_addresses"]
        }
        for lib in sidecar["data"]
    }
    libraries = [
        symbols.get((lib["debugName"], lib["codeId"]), {}) for lib in profile["libs"]
    ]

    (thread,) = profile["threads"]
    frames = thread["frameTable"]
    funcs = thread["funcTable"]
    resources = thread["resourceTable"]
    names = thread["stringArray"]
    frame_names = []
    for address, func in zip(frames["address"], frames["func"]):
        resource = funcs["resource"][func]
        lib = resources["lib"][resource] if resource >= 0 else None
        name = libraries[lib].get(address) if lib is not None and lib >= 0 else None
        frame_names.append(name or names[funcs["name"][func]])

    stacks = thread["stackTable"]
    samples = thread["samples"]
    stack_counts = Counter(samples["stack"])
    stack_cpu_us = Counter()
    for stack, delta in zip(samples["stack"], samples["threadCPUDelta"]):
        stack_cpu_us[stack] += delta or 0
    counts = Counter()
    cpu_us = Counter()
    leaves = defaultdict(Counter)
    for original_stack, count in stack_counts.items():
        stack = original_stack
        path = []
        while stack is not None:
            path.append(frame_names[stacks["frame"][stack]])
            stack = stacks["prefix"][stack]
        category = group(path)
        counts[category] += count
        cpu_us[category] += stack_cpu_us[original_stack]
        leaves[category][path[0] if path else "(empty stack)"] += count

    print("Main-thread sample count\t" + str(sum(counts.values())))
    print("First sample from recording start (ms)\t" + str(samples["time"][0]))
    print("Last sample from recording start (ms)\t" + str(samples["time"][-1]))
    print("Recorded thread CPU deltas (µs)\t" + str(sum(cpu_us.values())))
    print("Category\tSamples\tThread CPU delta (µs)\tMost sampled leaf\tLeaf samples")
    for category, count in counts.most_common():
        leaf, leaf_count = leaves[category].most_common(1)[0]
        print(f"{category}\t{count}\t{cpu_us[category]}\t{leaf}\t{leaf_count}")


def heap_summary(directory):
    profile = json.loads((directory / "dhat-heap.json").read_text())
    groups = defaultdict(Counter)
    for point in profile["pps"]:
        category = group([profile["ftbl"][index] for index in point["fs"]])
        for metric in ("tb", "tbk", "gb"):
            groups[category][metric] += point[metric]

    def row(category, values):
        print(category, *(values[metric] for metric in ("tb", "tbk", "gb")), sep="\t")

    print(f"DHAT capture duration (µs)\t{profile['te']}")
    print("Category\tCumulative bytes\tCumulative blocks\tLive bytes at global heap peak")
    row("Total", sum(groups.values(), Counter()))
    for category, values in sorted(groups.items(), key=lambda entry: -entry[1]["tb"]):
        row(category, values)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("mode", choices=("cpu", "heap"))
    parser.add_argument("run_directory", type=Path)
    args = parser.parse_args()
    {"cpu": cpu_summary, "heap": heap_summary}[args.mode](args.run_directory)


if __name__ == "__main__":
    main()
