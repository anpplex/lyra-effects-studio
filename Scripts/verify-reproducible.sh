#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
TEMP_ROOT="$(mktemp -d)"
cleanup() { rm -rf "$TEMP_ROOT"; }
trap cleanup EXIT

before_status="$(git -C "$REPO_ROOT" status --porcelain=v1 --untracked-files=all)"
test_key="$(openssl rand -base64 32 | tr -d '\n')"
source_epoch="$(git -C "$REPO_ROOT" show -s --format=%ct HEAD)"

for run in one two; do
  LYRA_REGISTRY_PRIVATE_KEY_BASE64="$test_key" \
  LYRA_REGISTRY_KEY_ID="reproducibility-test" \
  SOURCE_DATE_EPOCH="$source_epoch" \
    bash "$SCRIPT_DIR/build-registry.sh" "$TEMP_ROOT/$run" >/dev/null
done

diff -qr --exclude='registry-v1.json' --exclude='registry-v1.sig' "$TEMP_ROOT/one" "$TEMP_ROOT/two"
jq 'del(.packs[].signature)' "$TEMP_ROOT/one/registry-v1.json" > "$TEMP_ROOT/one.normalized.json"
jq 'del(.packs[].signature)' "$TEMP_ROOT/two/registry-v1.json" > "$TEMP_ROOT/two.normalized.json"
diff -u "$TEMP_ROOT/one.normalized.json" "$TEMP_ROOT/two.normalized.json"
"$REPO_ROOT/target/release/lyra-effects" registry verify-site "$TEMP_ROOT/one" >/dev/null
"$REPO_ROOT/target/release/lyra-effects" registry verify-site "$TEMP_ROOT/two" >/dev/null
after_status="$(git -C "$REPO_ROOT" status --porcelain=v1 --untracked-files=all)"
if [[ "$before_status" != "$after_status" ]]; then
  echo "Registry build modified the source tree." >&2
  diff <(printf '%s\n' "$before_status") <(printf '%s\n' "$after_status") || true
  exit 1
fi

file_count="$(find "$TEMP_ROOT/one" -type f | wc -l | tr -d ' ')"
echo "Registry reproducibility verified across $file_count files."
