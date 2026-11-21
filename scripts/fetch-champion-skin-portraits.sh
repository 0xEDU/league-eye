#!/usr/bin/env bash
set -euo pipefail

champion="${1:?champion name is required}"
output_dir="${2:-assets/skin-portraits}"
normalized="$(printf '%s' "$champion" | tr '[:upper:]' '[:lower:]')"
case "$normalized" in
  *[!a-z]*) echo "champion name must use letters only" >&2; exit 1 ;;
esac
mkdir -p "$output_dir/$normalized"
base_url="https://raw.communitydragon.org/latest/game/assets/characters/$normalized/hud"
listing_url="https://raw.communitydragon.org/json/latest/game/assets/characters/$normalized/hud/"
curl --fail --location --silent --show-error "$listing_url" \
  | jq -r --arg prefix "${normalized}_circle_" '.[] | select(.type == "file" and (.name | startswith($prefix)) and (.name | endswith(".png"))) | .name' \
  | while read -r image; do
    curl --fail --location --silent --show-error "$base_url/$image" --output "$output_dir/$normalized/$image"
  done
