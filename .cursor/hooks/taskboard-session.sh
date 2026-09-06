#!/usr/bin/env bash
# Inject the current Taskboard into the agent session. Fail open.
cat >/dev/null || true

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"

tb_cli() {
  if command -v tb >/dev/null 2>&1; then
    command tb --actor cursor "$@"
  elif command -v taskboard >/dev/null 2>&1; then
    command taskboard --actor cursor "$@"
  elif [[ -x "$ROOT/target/debug/taskboard" ]]; then
    "$ROOT/target/debug/taskboard" --actor cursor "$@"
  else
    return 127
  fi
}

context='Track implementation work on the Taskboard CLI (see AGENTS.md). Use --json --actor cursor. Start a run before the first edit; finish or fail it before the final reply. run finish does not move the card.'

if board="$(tb_cli task list --project taskboard --json 2>/dev/null)" && [[ -n "$board" ]]; then
  context+=$'\n\nCurrent taskboard tasks:\n'"$board"
elif projects="$(tb_cli project list --json 2>/dev/null)" && [[ -n "$projects" ]]; then
  context+=$'\n\nTaskboard projects:\n'"$projects"$'\nCreate or reuse project slug taskboard for this repo.'
fi

if command -v python3 >/dev/null 2>&1; then
  python3 -c 'import json,sys; print(json.dumps({"additional_context": sys.stdin.read()}))' <<<"$context"
else
  printf '%s\n' '{}'
fi
