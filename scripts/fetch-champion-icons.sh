#!/usr/bin/env bash
set -euo pipefail

output_dir="${1:-assets/champions}"
version="${2:-16.23.1}"
mkdir -p "$output_dir"
metadata="$output_dir/champion.json"
curl --fail --location --silent --show-error "https://ddragon.leagueoflegends.com/cdn/$version/data/en_US/champion.json" --output "$metadata"
jq -r '.data[] | .id + " " + .image.full' "$metadata" | while read -r champion image; do
  curl --fail --location --silent --show-error "https://ddragon.leagueoflegends.com/cdn/$version/img/champion/$image" --output "$output_dir/$champion.png"
done
