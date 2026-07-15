#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
OUTPUT_INPUT="${1:-$REPO_ROOT/.build/registry-site}"
mkdir -p "$(dirname "$OUTPUT_INPUT")"
OUTPUT_DIR="$(cd "$(dirname "$OUTPUT_INPUT")" && pwd)/$(basename "$OUTPUT_INPUT")"

if [[ "$OUTPUT_DIR" == "/" || "$OUTPUT_DIR" == "$REPO_ROOT" ]]; then
  echo "Refusing unsafe Registry output directory: $OUTPUT_DIR" >&2
  exit 64
fi

KEY_FILE="${LYRA_REGISTRY_PRIVATE_KEY_FILE:-}"
TEMP_KEY_FILE=""
cleanup() {
  [[ -z "$TEMP_KEY_FILE" ]] || rm -f "$TEMP_KEY_FILE"
}
trap cleanup EXIT

if [[ -z "$KEY_FILE" && -n "${LYRA_REGISTRY_PRIVATE_KEY_BASE64:-}" ]]; then
  TEMP_KEY_FILE="$(mktemp)"
  printf '%s\n' "$LYRA_REGISTRY_PRIVATE_KEY_BASE64" > "$TEMP_KEY_FILE"
  chmod 600 "$TEMP_KEY_FILE"
  KEY_FILE="$TEMP_KEY_FILE"
fi
if [[ -z "$KEY_FILE" || ! -f "$KEY_FILE" ]]; then
  echo "Set LYRA_REGISTRY_PRIVATE_KEY_FILE or LYRA_REGISTRY_PRIVATE_KEY_BASE64." >&2
  exit 66
fi

SOURCE_DATE_EPOCH="${SOURCE_DATE_EPOCH:-$(git -C "$REPO_ROOT" show -s --format=%ct HEAD)}"
GENERATED_AT="$(python3 -c 'import datetime,sys; print(datetime.datetime.fromtimestamp(int(sys.argv[1]), datetime.timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ"))' "$SOURCE_DATE_EPOCH")"
REGISTRY_ID="${LYRA_REGISTRY_ID:-org.lyra.effects.official}"
REGISTRY_NAME="${LYRA_REGISTRY_NAME:-Lyra Official Effects}"
KEY_ID="${LYRA_REGISTRY_KEY_ID:-lyra-official-v1}"

rm -rf "$OUTPUT_DIR"
mkdir -p "$OUTPUT_DIR/packs"
cargo build --manifest-path "$REPO_ROOT/Cargo.toml" --release --bin lyra-effects >/dev/null
CLI="$REPO_ROOT/target/release/lyra-effects"

packs='[]'
for pack_dir in "$REPO_ROOT"/Registry/Packs/*; do
  manifest="$pack_dir/lyra-pack.json"
  id="$(jq -er '.id' "$manifest")"
  name="$(jq -er '.name' "$manifest")"
  family="$(jq -er '.family' "$manifest")"
  version="$(jq -er '.version' "$manifest")"
  theme_id=""
  if [[ "$family" == "better-lyrics" ]]; then
    theme_id="$(jq -er '.entry.themeId | strings | select(test("^[a-z0-9](?:[a-z0-9-]*[a-z0-9])?$"))' "$manifest")" || {
      echo "Better Lyrics Pack is missing a valid entry.themeId: $manifest" >&2
      exit 65
    }
  fi
  version_dir="$OUTPUT_DIR/packs/$id/$version"
  archive="$version_dir/pack.lyra-pack.zip"
  mkdir -p "$version_dir"

  pack_result="$("$CLI" pack "$pack_dir" "$archive")"
  sha256="$(jq -er '.data.sha256' <<<"$pack_result")"
  size="$(jq -er '.data.byteCount' <<<"$pack_result")"
  sign_result="$("$CLI" registry sign-checksum "$sha256" "$KEY_FILE")"
  signature="$(jq -er '.data.signature' <<<"$sign_result")"
  cp "$manifest" "$version_dir/lyra-pack.json"

  entry="$(jq -cn \
    --arg id "$id" --arg name "$name" --arg family "$family" --arg version "$version" --arg themeId "$theme_id" \
    --arg manifestUrl "packs/$id/$version/lyra-pack.json" \
    --arg downloadUrl "packs/$id/$version/pack.lyra-pack.zip" \
    --arg sha256 "$sha256" --arg signature "$signature" --argjson size "$size" \
    '{id:$id,name:$name,family:$family,version:$version} + (if $themeId == "" then {} else {themeId:$themeId} end) + {manifestUrl:$manifestUrl,downloadUrl:$downloadUrl,sha256:$sha256,signature:$signature,size:$size,minimumRuntimeApi:"1.0.0"}')"
  packs="$(jq -cn --argjson packs "$packs" --argjson entry "$entry" '$packs + [$entry]')"
done

unsigned_catalog="$OUTPUT_DIR/.registry-v1.unsigned.json"
jq -cn \
  --arg registryId "$REGISTRY_ID" --arg name "$REGISTRY_NAME" --arg generatedAt "$GENERATED_AT" \
  --arg keyId "$KEY_ID" --argjson packs "$packs" \
  '{schemaVersion:1,registryId:$registryId,name:$name,generatedAt:$generatedAt,keyId:$keyId,packs:$packs}' \
  > "$unsigned_catalog"

"$CLI" registry build "$unsigned_catalog" "$OUTPUT_DIR" "$KEY_FILE" >/dev/null
rm -f "$unsigned_catalog"
"$CLI" registry verify-site "$OUTPUT_DIR" >/dev/null

printf '{"output":"%s","packCount":%s,"generatedAt":"%s"}\n' "$OUTPUT_DIR" "$(jq '.packs | length' "$OUTPUT_DIR/registry-v1.json")" "$GENERATED_AT"
