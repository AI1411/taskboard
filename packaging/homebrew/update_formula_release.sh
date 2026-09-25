#!/bin/sh
# Rewrite Formula version and sha256 from a published macOS tarball.
# Usage: update_formula_release.sh VERSION SHA256 [FORMULA]
# VERSION may be 0.1.2 or v0.1.2. SHA256 must be 64 lowercase hex chars.
# A rejected sha leaves FORMULA unchanged.
set -eu

version=${1:?usage: update_formula_release.sh VERSION SHA256 [FORMULA]}
sha=${2:?usage: update_formula_release.sh VERSION SHA256 [FORMULA]}
formula=${3:-Formula/taskboard.rb}

case "$version" in
  v*) version=${version#v} ;;
esac

if ! printf '%s\n' "$sha" | grep -Eq '^[0-9a-f]{64}$'; then
  printf 'update_formula_release: sha256 must be 64 hex chars\n' >&2
  exit 1
fi

tmp=$(mktemp)
awk -v version="$version" -v sha="$sha" '
  /^  version "/ { print "  version \"" version "\""; next }
  /^  sha256 "/ { print "  sha256 \"" sha "\""; next }
  { print }
' "$formula" >"$tmp"
mv "$tmp" "$formula"
