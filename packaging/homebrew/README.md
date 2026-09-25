# Homebrew tap `AI1411/taskboard`

Formula name: `taskboard`. Supported release target: Apple silicon, macOS 14+.

## Install

Preferred (tap repo `https://github.com/AI1411/homebrew-taskboard`):

```bash
brew tap AI1411/taskboard
brew install taskboard
```

If that GitHub repo is not available yet, tap this repository by URL (it contains `Formula/taskboard.rb`):

```bash
brew tap AI1411/taskboard https://github.com/AI1411/taskboard
brew install taskboard
```

## Tarball

`packaging/homebrew/package_cli_tarball.sh` packs a `taskboard` binary plus a `tb` symlink. The tag workflow `.github/workflows/release-cli.yml` packs that layout for `aarch64-apple-darwin`, `x86_64-unknown-linux-musl`, and `aarch64-unknown-linux-gnu`, and uploads each tarball with a `.sha256`. `Formula/taskboard.rb` `sha256` must match the macOS GitHub Release asset. The workflow opens a pull request with that hash; do not invent one:

```bash
shasum -a 256 taskboard-aarch64-apple-darwin.tar.gz
```

## HEAD

`brew install --HEAD` runs `pnpm install --frozen-lockfile` and `pnpm --filter web build` before `cargo install`, so the embedded UI matches a release build. A HEAD install needs Homebrew `node` and `pnpm`.

## `tb` alias

`taskboard` always installs. `tb` is created only when that name is absent or already points at `taskboard`. A foreign `tb` is left in place and the installer prints `alias_skipped`.

- Homebrew: `Formula/taskboard.rb` `post_install`
- Tarball / prefix installs: `packaging/homebrew/install_tb_alias.sh BINDIR`

## Local MCP

Same stdio snippets as the root README (`command: tb` or `taskboard`, `args: ["mcp"]`, `TASKBOARD_ACTOR`). Copy:

- Cursor: `packaging/mcp/cursor.mcp.json` → `.cursor/mcp.json`
- Claude Code: `packaging/mcp/claude.mcp.json` → `.mcp.json`
- Codex: `packaging/mcp/codex.config.toml` → `~/.codex/config.toml`

Homebrew `caveats` repeat this. Smoke: `tools/list` includes `next`, `review`, `run_cancel`.

```bash
printf '%s\n' '{"jsonrpc":"2.0","id":1,"method":"tools/list","params":{}}' | tb mcp
```

## Tests

```bash
sh packaging/homebrew/tests/run.sh
```
