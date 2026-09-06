#!/bin/sh
# Build the CLI release tarball: top-level `taskboard` plus `tb` -> taskboard.
# Usage: package_cli_tarball.sh /path/to/taskboard OUT.tar.gz
set -eu

binary=${1:?usage: package_cli_tarball.sh BINARY OUT.tar.gz}
out=${2:?usage: package_cli_tarball.sh BINARY OUT.tar.gz}

if [ ! -f "$binary" ]; then
  printf 'package_cli_tarball: not a file: %s\n' "$binary" >&2
  exit 1
fi

stage=$(mktemp -d)
trap 'rm -rf "$stage"' EXIT

cp "$binary" "$stage/taskboard"
chmod 755 "$stage/taskboard"
ln -s taskboard "$stage/tb"

tar -C "$stage" -czf "$out" taskboard tb
