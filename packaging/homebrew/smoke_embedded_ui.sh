#!/bin/sh
# Fail unless `taskboard serve` returns HTML that includes a <script tag.
# Usage: smoke_embedded_ui.sh /path/to/taskboard [port]
set -eu

binary=${1:?usage: smoke_embedded_ui.sh BINARY [port]}
port=${2:-47921}
data=$(mktemp -d)
pid=""
cleanup() {
  if [ -n "$pid" ]; then
    kill "$pid" 2>/dev/null || true
  fi
  rm -rf "$data"
}
trap cleanup EXIT

"$binary" --data-dir "$data" serve --port "$port" >"$data/serve.log" 2>&1 &
pid=$!

i=0
while [ "$i" -lt 50 ]; do
  if curl -fsS "http://127.0.0.1:${port}/" | grep -q '<script'; then
    exit 0
  fi
  i=$((i + 1))
  sleep 0.2
done

echo "served page has no <script" >&2
cat "$data/serve.log" >&2 || true
exit 1
