#!/usr/bin/env bash
set -euo pipefail

if [[ $# != 3 || ! $1 =~ ^(wall|cpu|heap)$ ]]; then
    echo "usage: $0 {wall|cpu|heap} TIER SEED" >&2
    exit 2
fi

cd "$(dirname "$0")/.."
mode=$1
tier=$2
seed=$3
mkdir -p target/profiling
run_dir=$(mktemp -d "$PWD/target/profiling/$mode-$tier-$seed.XXXXXX")

export CARGO_TARGET_DIR="$PWD/target/profiling/build-$mode"
export CARGO_PROFILE_RELEASE_DEBUG=1
export CARGO_PROFILE_RELEASE_STRIP=false
export RUSTFLAGS="-C force-frame-pointers=yes"

build=(cargo build --locked --release -p xtask -p convex-typegen-bench --bin xtask --bin profile-pipeline)
if [[ $mode == heap ]]; then
    build+=(--features convex-typegen-bench/profile-heap)
fi

set -x
git rev-parse HEAD
uname -sm
if [[ $(uname -s) == Darwin ]]; then
    sw_vers
    sysctl hw.model machdep.cpu.brand_string hw.memsize
fi
rustc -Vv
"${build[@]}"
bin="$CARGO_TARGET_DIR/release"
"$bin/xtask" "$tier" --seed "$seed" --out "$run_dir/project"
find "$run_dir/project/convex" -type f -name '*.ts' | wc -l

cd "$run_dir"
case $mode in
    wall|heap)
        "$bin/profile-pipeline" project convex_types.rs
        ;;
    cpu)
        # Keep the sampled executable available if Cargo rebuilds during a long capture.
        cp -p "$bin/profile-pipeline" ./profile-pipeline
        samply --version
        record=(samply record --save-only --unstable-presymbolicate --main-thread-only --rate 99)
        if [[ -n ${PROFILE_CPU_SECONDS:-} ]]; then
            record+=(--duration "$PROFILE_CPU_SECONDS")
        fi
        "${record[@]}" --output cpu.json.gz "$run_dir/profile-pipeline" project convex_types.rs
        if [[ -n ${PROFILE_CPU_SECONDS:-} && ! -f convex_types.rs ]]; then
            echo "Capture ended before the Rust file was written." >&2
            exit 0
        fi
        ;;
esac
wc -c convex_types.rs
