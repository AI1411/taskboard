#!/bin/sh
# Create bindir/tb -> taskboard unless a foreign `tb` is already present.
# Prints `alias_skipped` when the alias is left untouched. Always exits 0
# when bindir/taskboard exists so installers can keep going (issue #24).
# Usage: install_tb_alias.sh BINDIR
set -eu

bin_dir=${1:?usage: install_tb_alias.sh BINDIR}
taskboard="$bin_dir/taskboard"
tb="$bin_dir/tb"

if [ ! -e "$taskboard" ]; then
  printf 'install_tb_alias: taskboard not found in %s\n' "$bin_dir" >&2
  exit 1
fi

points_at_taskboard() {
  tb_real=$(realpath "$tb" 2>/dev/null) || return 1
  taskboard_real=$(realpath "$taskboard") || return 1
  [ "$tb_real" = "$taskboard_real" ]
}

if [ -L "$tb" ] || [ -e "$tb" ]; then
  if points_at_taskboard; then
    exit 0
  fi
  printf '%s\n' 'alias_skipped'
  exit 0
fi

ln -s taskboard "$tb"
