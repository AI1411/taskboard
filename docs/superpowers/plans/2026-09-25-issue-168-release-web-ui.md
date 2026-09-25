# Release CLI includes the web UI Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make release and Homebrew HEAD builds embed the Vite web UI, smoke `<script` before publishing, and cut `v0.1.1` so Homebrew users get a real board.

**Architecture:** `crates/api/build.rs` copies `apps/web/dist` into `$OUT_DIR/web-dist`, which `include_dir!` embeds. Release profile (`PROFILE=release`) panics when `index.html` is missing instead of writing the blank `HTML_SHELL`. Debug builds keep the shell so `cargo test` does not need Node. `release-cli.yml` builds the web app with pnpm before `cargo build --release`, then runs `packaging/homebrew/smoke_embedded_ui.sh`. The Formula HEAD path runs the same pnpm build before `cargo install`. Tag `v0.1.1` only after this lands on `main`. Update `Formula/taskboard.rb` `version` and `sha256` from the published asset; do not invent a hash.

**Tech Stack:** Rust build script, pnpm 10.33.3, Node 22, GitHub Actions `macos-14`, Homebrew Formula.

## Global Constraints

- Four columns stay `todo | in-progress | in-review | done`
- `run finish` does not move the card
- HTTP is for people; agents use CLI/MCP
- Do not invent a sha256
- If tag push or the GitHub Release is blocked, land the workflow and Formula HEAD fix and report the blocker
- Linux tarballs, Intel Mac, Windows, and desktop `.dmg` stay out of this issue
- Keep the existing Origin check; this issue does not change Host validation

User already chose sequential inline execution.

## File map

- Create: `crates/api/src/embed_policy.rs` — pure decision for copy vs shell vs error
- Create: `crates/api/tests/embed_policy.rs` — unit tests for that decision
- Modify: `crates/api/build.rs` — call the policy; panic on release without dist
- Create: `packaging/homebrew/smoke_embedded_ui.sh` — curl `/` and require `<script`
- Modify: `.github/workflows/release-cli.yml` — Node, pnpm, web build, smoke
- Modify: `Formula/taskboard.rb` — HEAD builds the web UI (sha256/version wait for the asset)
- Modify: `packaging/homebrew/tests/run.sh` — assert workflow, smoke script, and HEAD steps
- Modify: `packaging/homebrew/README.md` — HEAD builds the web UI
- Modify: `CHANGELOG.md` — move Unreleased into `[0.1.1]`

---

### Task 1: Embed policy fails release builds that lack dist

**Files:**
- Create: `crates/api/src/embed_policy.rs`
- Create: `crates/api/tests/embed_policy.rs`
- Modify: `crates/api/build.rs`

**Interfaces:**
- Consumes: `PROFILE` and whether `apps/web/dist/index.html` exists
- Produces: `pub enum EmbedChoice { CopyDist, HtmlShell }` and `pub fn decide(profile: &str, index_exists: bool) -> Result<EmbedChoice, &'static str>`

- [ ] **Step 1: Write the failing test**

```rust
#[path = "../src/embed_policy.rs"]
mod embed_policy;

use embed_policy::{decide, EmbedChoice};

#[test]
fn release_without_dist_is_an_error() {
    let err = decide("release", false).unwrap_err();
    assert!(err.contains("apps/web/dist/index.html"));
}

#[test]
fn debug_without_dist_uses_the_blank_shell() {
    assert_eq!(decide("debug", false).unwrap(), EmbedChoice::HtmlShell);
}

#[test]
fn any_profile_with_dist_copies_it() {
    assert_eq!(decide("release", true).unwrap(), EmbedChoice::CopyDist);
    assert_eq!(decide("debug", true).unwrap(), EmbedChoice::CopyDist);
}
```

- [ ] **Step 2: Run the test and watch it fail to compile**

Run: `cargo test -p taskboard-api --test embed_policy`

Expected: FAIL, `embed_policy.rs` not found.

- [ ] **Step 3: Implement the policy and wire `build.rs`**

`crates/api/src/embed_policy.rs`:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EmbedChoice {
    CopyDist,
    HtmlShell,
}

pub fn decide(profile: &str, index_exists: bool) -> Result<EmbedChoice, &'static str> {
    if index_exists {
        Ok(EmbedChoice::CopyDist)
    } else if profile == "release" {
        Err("release build requires apps/web/dist/index.html; run pnpm --filter web build first")
    } else {
        Ok(EmbedChoice::HtmlShell)
    }
}
```

`crates/api/build.rs` replaces the `if vite_dist.join("index.html").is_file()` branch with:

```rust
#[path = "src/embed_policy.rs"]
mod embed_policy;

use embed_policy::{decide, EmbedChoice};

// inside main(), after creating `out`:
let profile = env::var("PROFILE").unwrap_or_default();
let index = vite_dist.join("index.html");
match decide(&profile, index.is_file()) {
    Ok(EmbedChoice::CopyDist) => copy_dir(&vite_dist, &out).expect("copy vite dist"),
    Ok(EmbedChoice::HtmlShell) => {
        println!(
            "cargo:warning=apps/web/dist/index.html missing; embedding the blank HTML shell"
        );
        fs::write(out.join("index.html"), HTML_SHELL).unwrap();
    }
    Err(message) => panic!("{message}"),
}
```

- [ ] **Step 4: Run the test**

Run: `cargo test -p taskboard-api --test embed_policy`

Expected: PASS, 3 tests.

- [ ] **Step 5: Commit**

```bash
git add crates/api/src/embed_policy.rs crates/api/tests/embed_policy.rs crates/api/build.rs
git commit -m "fix(api): refuse release builds that embed a blank web shell"
```

---

### Task 2: Release workflow builds the web UI and smokes it

**Files:**
- Create: `packaging/homebrew/smoke_embedded_ui.sh`
- Modify: `.github/workflows/release-cli.yml`
- Modify: `packaging/homebrew/tests/run.sh`
- Modify: `Formula/taskboard.rb`
- Modify: `packaging/homebrew/README.md`
- Modify: `CHANGELOG.md`

**Interfaces:**
- Consumes: `decide` from Task 1 (release binary embeds dist only when the web build ran)
- Produces: workflow step order Node → pnpm install → `pnpm --filter web build` → `cargo build --release` → `smoke_embedded_ui.sh`; Formula HEAD runs the same pnpm commands before `cargo install`

- [ ] **Step 1: Add failing packaging asserts**

Before `if [ "$fail" -ne 0 ]` in `packaging/homebrew/tests/run.sh`:

```sh
assert "release workflow builds the web UI" \
  grep -q 'pnpm --filter web build' "$repo/.github/workflows/release-cli.yml"
assert "release workflow smokes the embedded UI" \
  grep -q 'smoke_embedded_ui.sh' "$repo/.github/workflows/release-cli.yml"
assert "smoke script requires a script tag" \
  grep -q '<script' "$repo/packaging/homebrew/smoke_embedded_ui.sh"
assert "formula HEAD builds the web UI" \
  grep -Fq 'system "pnpm", "--filter", "web", "build"' "$formula"
```

- [ ] **Step 2: Run packaging tests and watch them fail**

Run: `sh packaging/homebrew/tests/run.sh`

Expected: `not ok` for the four new asserts. Earlier asserts still pass.

- [ ] **Step 3: Add the smoke script**

`packaging/homebrew/smoke_embedded_ui.sh`:

```sh
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
```

`chmod +x` the script.

- [ ] **Step 4: Build the web UI in `release-cli.yml` before cargo**

After checkout and the Rust toolchain step, before `Build CLI`:

```yaml
      - uses: actions/setup-node@v4
        with:
          node-version: "22"
      - name: Enable pnpm
        run: |
          corepack enable
          corepack prepare pnpm@10.33.3 --activate
      - name: Build web UI
        run: |
          pnpm install --frozen-lockfile
          pnpm --filter web build
```

After `Build CLI`, before `Pack tarball`:

```yaml
      - name: Smoke embedded web UI
        run: sh packaging/homebrew/smoke_embedded_ui.sh target/release/taskboard
```

- [ ] **Step 5: Formula HEAD builds the web UI**

In `Formula/taskboard.rb`, add build dependencies next to the rust head dep:

```ruby
  depends_on "node" => :build if build.head?
  depends_on "pnpm" => :build if build.head?
```

Replace the head branch of `install`:

```ruby
    if build.head?
      system "pnpm", "install", "--frozen-lockfile"
      system "pnpm", "--filter", "web", "build"
      system "cargo", "install", "--locked", "--root", prefix, "--path", "crates/cli"
    else
```

Append to the caveats heredoc, before the closing `EOS`:

```
      HEAD installs build the web UI (node, pnpm, then cargo) so `tb serve`
      is not a blank page. Release bottles embed that UI.
```

- [ ] **Step 6: Document HEAD and date the changelog**

In `packaging/homebrew/README.md`, after the tarball section, add:

```markdown
## HEAD

`brew install --HEAD` runs `pnpm install --frozen-lockfile` and `pnpm --filter web build` before `cargo install`, so the embedded UI matches a release build. A HEAD install needs Homebrew `node` and `pnpm`.
```

Move the current `## [Unreleased]` bullets into `## [0.1.1] - 2026-09-25` and add:

```markdown
- Release CLI and Homebrew HEAD embed the built web UI. A release build fails when `apps/web/dist/index.html` is missing, and the tag workflow refuses a binary whose `/` has no `<script`.
```

Leave `## [Unreleased]` empty.

- [ ] **Step 7: Re-run packaging tests**

Run: `sh packaging/homebrew/tests/run.sh`

Expected: `all packaging/homebrew tests passed`.

- [ ] **Step 8: Commit**

```bash
git add packaging/homebrew .github/workflows/release-cli.yml Formula/taskboard.rb CHANGELOG.md
git commit -m "fix(release): build and smoke the web UI before publishing the CLI"
```

---

### Task 3: Prove a release binary serves `<script`, then cut v0.1.1

**Files:**
- Modify (only after the GitHub Release asset exists): `Formula/taskboard.rb` `version` and `sha256`

**Interfaces:**
- Consumes: merged workflow on `origin/main`
- Produces: Release `v0.1.1` asset `taskboard-aarch64-apple-darwin.tar.gz`, then a Formula whose sha256 matches that asset

- [ ] **Step 1: Local release smoke**

```bash
corepack enable
corepack prepare pnpm@10.33.3 --activate
pnpm install --frozen-lockfile
pnpm --filter web build
cargo build --release -p taskboard-cli
sh packaging/homebrew/smoke_embedded_ui.sh target/release/taskboard 47921
```

Expected: exit 0. `curl -fsS http://127.0.0.1:47921/` contains `<script`.

Also confirm the guard, with dist renamed aside:

```bash
mv apps/web/dist apps/web/dist.bak
cargo build --release -p taskboard-api
```

Expected: panic containing `apps/web/dist/index.html`. Restore `apps/web/dist` afterward. Do not commit `apps/web/dist`.

- [ ] **Step 2: Open the PR, wait for CI, merge, delete the branch**

The PR explains #168. Do not tag from the feature branch.

- [ ] **Step 3: Tag `v0.1.1` on the merge commit**

```bash
git fetch origin main
git tag -a v0.1.1 origin/main -m "v0.1.1"
git push origin v0.1.1
gh run list --workflow release-cli.yml --limit 1
```

Expected: `macos-14` job success and a Release asset. If the tag push or the workflow is denied, stop. Do not write a sha256.

- [ ] **Step 4: Put the real hash in the Formula**

```bash
gh release download v0.1.1 --pattern 'taskboard-aarch64-apple-darwin.tar.gz' --dir /tmp/tb-rel
shasum -a 256 /tmp/tb-rel/taskboard-aarch64-apple-darwin.tar.gz
```

Set `version "0.1.1"` and that exact `sha256` in `Formula/taskboard.rb`. Mirror the Formula to `AI1411/homebrew-taskboard` if that repo is writable. Commit on a follow-up branch, PR, merge.

## Self-review

- Spec: workflow web build, smoke, release-profile failure, HEAD formula, v0.1.1 changelog — Tasks 1–3.
- No placeholder hashes. Formula version stays `0.1.0` until Step 4 of Task 3.
- `decide` signature matches Task 1 and the `build.rs` call.
