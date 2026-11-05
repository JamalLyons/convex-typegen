#!/usr/bin/env bash
set -Eeuo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
fixture_dir="$repo_root/tests/fixtures/v1-review"
artifact_dir="${E2E_ARTIFACT_DIR:-$repo_root/target/e2e-artifacts}"
backend_image="ghcr.io/convex/convex-backend@sha256:e9c084e8bd6c1f249012c672193f0988c748824f55a24cdad45f6185a8462f9a"
backend_name="convex-typegen-e2e-$$"
backend_url="http://127.0.0.1:13210"
backend_key=""
spec_file=""

mkdir -p "$artifact_dir"

sanitize_logs() {
    local raw_log="$artifact_dir/backend.raw.log"
    if docker container inspect "$backend_name" >/dev/null 2>&1; then
        docker logs "$backend_name" >"$raw_log" 2>&1 || true
        BACKEND_KEY_TO_REDACT="$backend_key" python3 - "$raw_log" "$artifact_dir/backend.log" <<'PY'
import os
import re
import sys

raw = open(sys.argv[1], encoding="utf-8", errors="replace").read()
key = os.environ.get("BACKEND_KEY_TO_REDACT", "")
if key:
    raw = raw.replace(key, "[REDACTED_ADMIN_KEY]")
raw = re.sub(r"convex-self-hosted\|[A-Za-z0-9]+", "[REDACTED_ADMIN_KEY]", raw)
raw = re.sub(r"(?i)(authorization:\s*bearer\s+)\S+", r"\1[REDACTED]", raw)
raw = re.sub(r"(?i)(admin[_ -]?key\s*[:=]\s*)\S+", r"\1[REDACTED]", raw)
open(sys.argv[2], "w", encoding="utf-8").write(raw)
PY
        rm -f "$raw_log"
    else
        : >"$artifact_dir/backend.log"
    fi
}

cleanup() {
    local exit_code=$?
    sanitize_logs || printf 'Backend logs could not be sanitized; see job output.\n' >"$artifact_dir/backend.log"
    docker rm --force "$backend_name" >/dev/null 2>&1 || true
    [[ -z "$spec_file" ]] || rm -f "$spec_file"
    return "$exit_code"
}
trap cleanup EXIT
trap 'exit 129' HUP
trap 'exit 130' INT
trap 'exit 143' TERM

for required in docker curl npm jq cargo; do
    command -v "$required" >/dev/null || { echo "required command not found: $required" >&2; exit 2; }
done

docker run --detach --name "$backend_name" \
    --publish 127.0.0.1:13210:3210 \
    --publish 127.0.0.1:13211:3211 \
    --env CONVEX_CLOUD_ORIGIN="$backend_url" \
    --env CONVEX_SITE_ORIGIN=http://127.0.0.1:13211 \
    --env DISABLE_BEACON=true \
    "$backend_image"

curl --fail --silent --retry 60 --retry-delay 1 --retry-connrefused --retry-all-errors --max-time 3 "$backend_url/version" >/dev/null
backend_key="$(docker exec "$backend_name" ./generate_admin_key.sh | tail -n 1)"
[[ -n "$backend_key" ]] || { echo "backend returned an empty ephemeral admin key" >&2; exit 1; }

export CONVEX_SELF_HOSTED_URL="$backend_url"
export CONVEX_SELF_HOSTED_ADMIN_KEY="$backend_key"
cd "$fixture_dir"
npm ci --no-audit --no-fund
npx --no-install convex dev --once --typecheck enable

spec_file="$(mktemp "${TMPDIR:-/tmp}/convex-typegen-function-spec.XXXXXX.json")"
npx --no-install convex function-spec >"$spec_file"
jq -e '.functions | type == "array" and length > 0' "$spec_file" >/dev/null
export CONVEX_FUNCTION_SPEC_PATH="$spec_file"

cd "$repo_root"
cargo run --offline --locked -p convex-typegen-e2e
