#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
fixture_dir="$repo_root/tests/fixtures/v1-review"
output="$fixture_dir/function-spec.json"

if [[ "${CONVEX_SELF_HOSTED_URL:-}" != "http://127.0.0.1:13210" ]]; then
  echo "CONVEX_SELF_HOSTED_URL must be http://127.0.0.1:13210" >&2
  exit 2
fi
if [[ -z "${CONVEX_SELF_HOSTED_ADMIN_KEY:-}" ]]; then
  echo "CONVEX_SELF_HOSTED_ADMIN_KEY must be set for the local backend" >&2
  exit 2
fi
command -v jq >/dev/null
command -v npm >/dev/null

raw="$(mktemp)"
canonical="$(mktemp "$fixture_dir/.function-spec.XXXXXX")"
trap 'rm -f "$raw" "$canonical"' EXIT

cd "$fixture_dir"
npx --no-install convex function-spec >"$raw"
jq -S '
  if (.functions | type) != "array" then error("expected .functions array") else . end
  | {functions: [.functions[] |
      if .functionType == "HttpAction" then
        {functionType, method, path}
      else
        {identifier, functionType, visibility, args, returns}
      end
    ] | sort_by([.functionType, (.identifier // ""), (.path // "")])}
' "$raw" >"$canonical"
mv "$canonical" "$output"
trap - EXIT
rm -f "$raw"
echo "Wrote $output"
