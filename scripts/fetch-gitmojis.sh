#!/bin/sh
set -eu
command -v jq >/dev/null 2>&1 || { echo "fetch-gitmojis: jq is not installed" >&2; exit 1; }

out="$(dirname "$0")/../data/gitmojis.json"
raw="$out.download"
trap 'rm -f "$raw" "$out.tmp"' EXIT

# A pipe would hide the exit code of curl, and an empty body would erase the data file.
curl -fsSL https://gitmoji.dev/api/gitmojis -o "$raw"
jq '[.gitmojis[] | {name: .code, description: (.description | rtrimstr(".")), emoji: .emoji}]' \
  "$raw" > "$out.tmp"

count="$(jq 'length' "$out.tmp")"
[ "$count" -gt 0 ] || { echo "fetch-gitmojis: the answer held no gitmojis" >&2; exit 1; }

mv "$out.tmp" "$out"
echo "fetch-gitmojis: wrote $count gitmojis"
