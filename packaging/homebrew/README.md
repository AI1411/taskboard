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

`packaging/homebrew/package_cli_tarball.sh` packs a `taskboard` binary plus a `tb` symlink. The tag workflow `.github/workflows/release-cli.yml` runs that script on `macos-14` and uploads the asset. `Formula/taskboard.rb` `sha256` must match that GitHub Release asset:

```bash
shasum -a 256 taskboard-aarch64-apple-darwin.tar.gz
```

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
