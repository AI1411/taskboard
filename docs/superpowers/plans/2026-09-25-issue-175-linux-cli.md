# Linux CLI tarballs and Formula automation Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Publish Linux CLI tarballs from the tag workflow, attach a `.sha256` for every tarball, and open a pull request that sets `Formula/taskboard.rb` version and sha256 from the macOS asset.

**Architecture:** `release-cli.yml` builds three release binaries (macOS arm64, Linux x86_64 musl, Linux aarch64 gnu), packs each with `package_cli_tarball.sh`, and uploads the tarballs plus checksum files in one publish job. A follow-up job checks out `main`, runs `update_formula_release.sh` with the sha256 of `taskboard-aarch64-apple-darwin.tar.gz`, and opens a PR. Agents without `tb` or `taskboard` on `PATH` use one curl line that picks the Linux asset. The committed Formula hash stays the v0.1.1 asset hash until that PR.

**Tech Stack:** GitHub Actions, `package_cli_tarball.sh`, Homebrew Formula, musl-gcc for `x86_64-unknown-linux-musl`.

## Global Constraints

- Four columns stay `todo | in-progress | in-review | done`
- One local SQLite database
- `run finish` does not move the card
- HTTP is for people; agents use CLI or MCP
- No new product behavior
- Do not invent a sha256
- Linux desktop, Intel Mac, and Windows stay out
- User already chose sequential inline execution

## File map

- Modify: `.github/workflows/release-cli.yml`
- Create: `packaging/homebrew/update_formula_release.sh`
- Modify: `packaging/homebrew/tests/run.sh`
- Modify: `packaging/homebrew/README.md`
- Modify: `AGENTS.md`
- Modify: `skills/using-taskboard/SKILL.md`
- Modify: `.cursor/skills/using-taskboard/SKILL.md`
- Modify: `.claude/skills/using-taskboard/SKILL.md`
- Modify: `.agents/skills/using-taskboard/SKILL.md`
- Modify: `CHANGELOG.md`

---

### Task 1: Workflow, formula updater, and Linux fetch line

**Files:**
- Test: `packaging/homebrew/tests/run.sh`
- Create: `packaging/homebrew/update_formula_release.sh`
- Modify: `.github/workflows/release-cli.yml`

**Interfaces:**
- Consumes: `package_cli_tarball.sh BINARY OUT.tar.gz`
- Produces: `update_formula_release.sh VERSION SHA256 [FORMULA]` rewrites `version` and `sha256` lines. `VERSION` may start with `v`. `SHA256` must be 64 hex chars or the script exits 1 and leaves the file unchanged.

- [ ] **Step 1: Write failing tests**

Append to `packaging/homebrew/tests/run.sh` before the `if [ "$fail" -ne 0 ]` block:

```sh
wf="$repo/.github/workflows/release-cli.yml"
assert "release workflow builds musl" grep -q 'x86_64-unknown-linux-musl' "$wf"
assert "release workflow builds arm gnu" grep -q 'aarch64-unknown-linux-gnu' "$wf"
assert "release workflow writes sha256 files" grep -q 'tar.gz.sha256' "$wf"
assert "release workflow updates the formula" grep -q 'update_formula_release.sh' "$wf"

formula_updater_sets_version_and_sha() {
  sample=$(mktemp)
  cp "$formula" "$sample"
  "$root/update_formula_release.sh" 9.9.9 0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef "$sample" || return 1
  grep -q 'version "9.9.9"' "$sample" || return 1
  grep -q 'sha256 "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"' "$sample" || return 1
  grep -q 'version "0.1.1"' "$formula" || return 1
}
assert "formula updater sets version and sha256 on a copy" formula_updater_sets_version_and_sha

updater_rejects_short_sha() {
  [ -f "$root/update_formula_release.sh" ] || return 1
  sample=$(mktemp)
  cp "$formula" "$sample"
  if "$root/update_formula_release.sh" 9.9.9 abc "$sample"; then
    return 1
  fi
  grep -q 'sha256 "ab06fbe9a7e5c09e51add912813d6962766844fb6f16b1abad5c58bbb4940a39"' "$sample"
}
assert "formula updater rejects a short sha" updater_rejects_short_sha

fetch='releases/latest/download/taskboard-${triple}.tar.gz'
assert "agents.md fetches the linux release" grep -Fq "$fetch" "$repo/AGENTS.md"
assert "agents.md names musl and arm gnu" grep -q 'x86_64-unknown-linux-musl' "$repo/AGENTS.md"
for skill in \
  skills/using-taskboard/SKILL.md \
  .cursor/skills/using-taskboard/SKILL.md \
  .claude/skills/using-taskboard/SKILL.md \
  .agents/skills/using-taskboard/SKILL.md
do
  assert "skill $skill fetches the linux release" grep -Fq "$fetch" "$repo/$skill"
done
```

- [ ] **Step 2: Run `sh packaging/homebrew/tests/run.sh`**

Expected: exit 1, `not ok` on the new asserts. Existing tarball asserts stay `ok`.

- [ ] **Step 3: Implement the updater, workflow, docs, and changelog**

`update_formula_release.sh` uses awk so the committed Formula is only rewritten when the release job calls it. The workflow matrix is `macos-14` / `aarch64-apple-darwin`, `ubuntu-latest` / `x86_64-unknown-linux-musl` (with `musl-tools` and `CC_x86_64_unknown_linux_musl=musl-gcc`), and `ubuntu-24.04-arm` / `aarch64-unknown-linux-gnu`. Each job builds the web UI, smokes `<script`, packs `taskboard` plus `tb`, and writes `ASSET.sha256`. Publish uploads every tarball and checksum. The formula job opens `release/formula-<tag>` against `main`.

Linux fetch line, inserted as resolution step 3 in `AGENTS.md` and every `using-taskboard` skill copy (cargo run becomes step 4):

```bash
triple=$(uname -s)-$(uname -m); case $triple in Linux-aarch64|Linux-arm64) triple=aarch64-unknown-linux-gnu ;; Linux-*) triple=x86_64-unknown-linux-musl ;; *) triple= ;; esac; [ -z "$triple" ] || curl -fsSL "https://github.com/AI1411/taskboard/releases/latest/download/taskboard-${triple}.tar.gz" | tar -xz
```

- [ ] **Step 4: Re-run `sh packaging/homebrew/tests/run.sh`**

Expected: `all packaging/homebrew tests passed`.

- [ ] **Step 5: Commit**
