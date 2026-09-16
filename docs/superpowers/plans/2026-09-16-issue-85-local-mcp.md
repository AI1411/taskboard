# Local MCP wrapping App

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** `tb mcp` speaks MCP over stdio and wraps the same `App` verbs the CLI uses. Agents must not call the localhost HTTP API.

**Architecture:** Newline-delimited JSON-RPC 2.0 on stdin/stdout. Tools call existing `App` methods with the CLI actor. First ship: project / task / run / inbox plus activity / comment / `run continue`.

**Tech Stack:** clap, serde_json, existing application crate. No new HTTP surface.

## Global Constraints

- Four columns stay `todo | in-progress | in-review | done`
- Finishing a run does not move the card
- Same validation and `--actor` as the CLI
- Do not open HTTP to agents
- CLI `--json` stays snake_case; MCP tool payloads reuse that shape

## File map

- `crates/cli/src/mcp.rs` — stdio JSON-RPC + tool dispatch
- `crates/cli/src/{args.rs,main.rs}` — `mcp` command
- `crates/cli/tests/{help.rs,mcp.rs}`
- `AGENTS.md` + skill — MCP if present, else CLI

User already chose sequential inline execution.
