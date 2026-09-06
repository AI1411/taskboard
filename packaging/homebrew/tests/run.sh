#!/bin/sh
# Tests for the CLI tarball layout and tb alias skip rules (issue #24).
set -eu

here=$(CDPATH= cd -- "$(dirname "$0")" && pwd)
root=$(CDPATH= cd -- "$here/.." && pwd)
repo=$(CDPATH= cd -- "$root/../.." && pwd)
formula="$repo/Formula/taskboard.rb"
fail=0

assert() {
  msg=$1
  shift
  if "$@"; then
    printf 'ok  %s\n' "$msg"
  else
    printf 'not ok  %s\n' "$msg"
    fail=1
  fi
}

tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT

printf '#!/bin/sh\necho fake-taskboard\n' >"$tmp/taskboard.bin"
chmod +x "$tmp/taskboard.bin"

"$root/package_cli_tarball.sh" "$tmp/taskboard.bin" "$tmp/taskboard-aarch64-apple-darwin.tar.gz"
list=$(tar -tzf "$tmp/taskboard-aarch64-apple-darwin.tar.gz" | LC_ALL=C sort | tr '\n' ' ')
assert "tarball contains taskboard and tb only" test "$list" = "taskboard tb "

mkdir "$tmp/extract"
tar -C "$tmp/extract" -xzf "$tmp/taskboard-aarch64-apple-darwin.tar.gz"
assert "taskboard is an executable file" test -f "$tmp/extract/taskboard" -a -x "$tmp/extract/taskboard"
assert "tb is a symlink" test -L "$tmp/extract/tb"
assert "tb points at taskboard" test "$(readlink "$tmp/extract/tb")" = "taskboard"

mkdir "$tmp/bin-empty"
cp "$tmp/taskboard.bin" "$tmp/bin-empty/taskboard"
chmod +x "$tmp/bin-empty/taskboard"
out=$("$root/install_tb_alias.sh" "$tmp/bin-empty")
assert "fresh install prints nothing" test -z "$out"
assert "fresh install creates tb symlink" test -L "$tmp/bin-empty/tb"
assert "fresh tb points at taskboard" test "$(readlink "$tmp/bin-empty/tb")" = "taskboard"

out=$("$root/install_tb_alias.sh" "$tmp/bin-empty")
assert "idempotent reinstall prints nothing" test -z "$out"
assert "idempotent reinstall keeps our tb" test -L "$tmp/bin-empty/tb"

mkdir "$tmp/bin-foreign"
cp "$tmp/taskboard.bin" "$tmp/bin-foreign/taskboard"
printf 'other-tb\n' >"$tmp/bin-foreign/tb"
out=$("$root/install_tb_alias.sh" "$tmp/bin-foreign")
assert "foreign tb prints alias_skipped" test "$out" = "alias_skipped"
assert "foreign tb contents unchanged" test "$(cat "$tmp/bin-foreign/tb")" = "other-tb"
assert "taskboard still installed beside foreign tb" test -f "$tmp/bin-foreign/taskboard"

mkdir "$tmp/bin-other-link"
cp "$tmp/taskboard.bin" "$tmp/bin-other-link/taskboard"
printf '#!/bin/sh\necho other\n' >"$tmp/bin-other-link/other"
chmod +x "$tmp/bin-other-link/other"
ln -s other "$tmp/bin-other-link/tb"
out=$("$root/install_tb_alias.sh" "$tmp/bin-other-link")
assert "foreign symlink prints alias_skipped" test "$out" = "alias_skipped"
assert "foreign symlink still points at other" test "$(readlink "$tmp/bin-other-link/tb")" = "other"

assert "formula exists" test -f "$formula"
assert "formula class is Taskboard" grep -q '^class Taskboard < Formula$' "$formula"
assert "formula is Apple silicon only" grep -q 'arch: :arm64' "$formula"
assert "formula requires macOS 14+" grep -q 'macos: :sonoma' "$formula"
assert "formula installs from aarch64-apple-darwin tarball" grep -q 'taskboard-aarch64-apple-darwin.tar.gz' "$formula"
assert "formula postinstall mentions alias_skipped" grep -q 'alias_skipped' "$formula"
assert "formula does not bin.install tb" awk '/def install/,/^  end$/{if ($0 ~ /bin.install .*tb/) exit 1}' "$formula"

if [ "$fail" -ne 0 ]; then
  printf '\npackaging/homebrew tests failed\n' >&2
  exit 1
fi

printf '\nall packaging/homebrew tests passed\n'
