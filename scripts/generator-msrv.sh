#!/usr/bin/env bash
set -euo pipefail

cargo +1.97.0 clippy --locked -p convex-typegen --all-targets --all-features -- -D warnings
CONVEX_TYPEGEN_UI_SNAPSHOTS=1 cargo +1.97.0 test --locked -p convex-typegen --all-features
cargo +1.97.0 build --locked -p convex-typegen --no-default-features

# Verify the build-dependency boundary with a standalone consumer.
fixture=$(mktemp -d)
trap 'rm -rf "$fixture"' EXIT
mkdir -p "$fixture/src" "$fixture/convex"
cat > "$fixture/Cargo.toml" <<EOF
[package]
name = "generator-smoke"
version = "0.0.0"
edition = "2024"
rust-version = "1.97"

[build-dependencies]
convex-typegen = { path = "$PWD" }
EOF
echo 'fn main() {}' > "$fixture/src/main.rs"
cat > "$fixture/build.rs" <<'EOF'
fn main() {
    let config = convex_typegen::config::Configuration::default();
    let output = config.out_file.clone();
    convex_typegen::generate(config).unwrap();
    assert!(std::fs::metadata(output).unwrap().len() > 0);
}
EOF
echo 'export default defineSchema({});' > "$fixture/convex/schema.ts"

# Reuse the tested generator resolution and keep the fixture separate.
cp Cargo.lock "$fixture/Cargo.lock"
check_graph() {
    local graph
    graph=$(cargo +1.97.0 tree --manifest-path "$fixture/Cargo.toml" --edges normal,build --prefix none "$@")
    if grep -E '^(convex(-typegen-runtime|_sync_types)?|native-tls|tokio-native-tls|openssl[^ ]*|rustls[^ ]*|tokio-rustls) v' <<< "$graph"; then
        exit 1
    fi
}
check_graph
check_graph --features convex-typegen/verbose
cargo +1.97.0 build --locked --manifest-path "$fixture/Cargo.toml"
