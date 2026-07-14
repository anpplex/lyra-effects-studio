#!/usr/bin/env bash
set -euo pipefail

MODE="${1:---check}"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
GIT_COMMON_DIR="$(git -C "$REPO_ROOT" rev-parse --path-format=absolute --git-common-dir)"
STUDIO_CHECKOUT="$(dirname "$GIT_COMMON_DIR")"
LYRA_ROOT="${LYRA_ROOT:-$(cd "$STUDIO_CHECKOUT/../Lyra" 2>/dev/null && pwd || true)}"

if [[ "$MODE" != "--check" && "$MODE" != "--sync" ]]; then
  echo "Usage: $0 [--check|--sync]" >&2
  exit 64
fi
if [[ -z "$LYRA_ROOT" || ! -f "$LYRA_ROOT/lyric-effects/packs/better-lyrics/themes/catalog.json" ]]; then
  echo "Set LYRA_ROOT to a Lyra checkout containing lyric-effects." >&2
  exit 66
fi

verify_theme() {
  local theme_id="$1"
  local pack_id="$2"
  local source="$LYRA_ROOT/lyric-effects/packs/better-lyrics/themes/$theme_id/lyra.css"
  local destination="$REPO_ROOT/Registry/Packs/$pack_id/theme/lyra.css"

  if [[ "$MODE" == "--sync" ]]; then
    mkdir -p "$(dirname "$destination")"
    cp "$source" "$destination"
  fi
  if ! cmp -s "$source" "$destination"; then
    echo "CSS mismatch: $theme_id -> $pack_id" >&2
    exit 65
  fi
}

verify_theme "dynamic-background" "io.github.chengggit.youtube-music-dynamic-theme"
verify_theme "modern-player" "io.github.snw-mint.better-lyrics-modern-player"
verify_theme "sustain" "io.github.better-lyrics.theme-sustain"

echo "Verified 3 license-cleared adapted themes."
