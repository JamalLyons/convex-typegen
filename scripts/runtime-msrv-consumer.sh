#!/usr/bin/env bash
set -euo pipefail

root="$PWD"
fixture="$(mktemp -d)"
trap 'rm -rf "$fixture"' EXIT
mkdir -p "$fixture/src"

# Run the generator separately. The external consumer has no build dependency.
cargo +1.97.0 run --locked -p convex-typegen --bin convex-typegen -- \
    --convex-dir tests/fixtures/v1-review/convex \
    --function-spec tests/fixtures/v1-review/function-spec.json \
    --output "$fixture/src/convex_api.rs"

cat > "$fixture/Cargo.toml" <<EOF_MANIFEST
[package]
name = "runtime-msrv-smoke"
version = "0.0.0"
edition = "2024"
rust-version = "1.85"

[dependencies]
convex-typegen-runtime = { path = "$root/runtime" }
EOF_MANIFEST
cp Cargo.lock "$fixture/Cargo.lock"
cat > "$fixture/src/lib.rs" <<'EOF_RUST'
#[cfg(test)]
mod convex_api;

#[test]
fn generated_enums_round_trip_on_runtime_msrv() {
    use convex_api::api::admin::users::{EchoArgsDataChoice, EchoReturnChoice};
    use convex_typegen_runtime::value_serde::{from_value, to_value};

    let wire = to_value(&EchoArgsDataChoice::Red).unwrap();
    assert_eq!(wire, to_value(&"red").unwrap());
    assert_eq!(from_value::<EchoReturnChoice>(wire).unwrap(), EchoReturnChoice::Red);
}
EOF_RUST
# Cargo prunes the copied workspace lockfile for this standalone runtime-only package.
cargo +1.85.0 test --offline --manifest-path "$fixture/Cargo.toml"
