# Project Detect Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Resolve the current project from `--project`, `TASKBOARD_PROJECT`, or a unique `repo_path` ancestor of `cwd`. Ship `tb project detect` and make `task create` / `task list` accept an omitted `--project`.

**Architecture:** `App::detect_project` is the single resolver. CLI `project detect` prints the Project. `task create` / `task list` call detect when `--project` is omitted. `tb inbox` does not use detect. `AGENTS.md` opening list switches to `project detect`.

**Tech Stack:** Rust (`taskboard-application`, `taskboard-cli`), clap, tempfile, assert_cmd. Spec §8: `docs/superpowers/specs/2026-09-06-usability-mechanisms-design.md`. Issue: https://github.com/AI1411/taskboard/issues/55. Existing plan Task 9.

## Global Constraints

- Four columns stay `todo | in-progress | in-review | done`
- Titles are never task identifiers; mutations still require `TASK-n`
- Inbox does **not** use this resolver; omitted `--project` still means all live projects
- Empty/missing `repo_path` never matches
- Unknown `TASKBOARD_PROJECT` is `not_found`, not `project_required`
- `project_required` message is exactly `pass --project or set TASKBOARD_PROJECT`
- No command palette, `tb work start`, or title-as-ID

## File map

- Create: `crates/application/src/detect.rs`
- Create: `crates/application/tests/detect.rs`
- Modify: `crates/application/src/lib.rs`
- Modify: `crates/application/src/error.rs`
- Modify: `crates/application/src/app.rs`
- Modify: `crates/cli/src/args.rs`
- Modify: `crates/cli/src/main.rs`
- Modify: `crates/cli/tests/cli_json.rs`
- Modify: `crates/cli/tests/help.rs`
- Modify: `crates/api/src/routes.rs` (exhaustive `AppError` match)
- Modify: `AGENTS.md`

---

### Task 1: detect_project, CLI detect, optional --project

**Files:**
- Create: `crates/application/src/detect.rs`, `crates/application/tests/detect.rs`
- Modify: `crates/application/src/{lib.rs,error.rs,app.rs}`
- Modify: `crates/cli/src/{args.rs,main.rs}`
- Modify: `crates/cli/tests/{cli_json.rs,help.rs}`
- Modify: `crates/api/src/routes.rs`
- Modify: `AGENTS.md`

**Interfaces:**
- Consumes: `App::project_list(false)`
- Produces:
  - `AppError::ProjectRequired` — `code() == "project_required"`, Display `pass --project or set TASKBOARD_PROJECT`
  - `pub async fn App::detect_project(&self, cwd: &Path, env_slug: Option<&str>) -> Result<Project, AppError>`
  - Resolution: `env_slug` if the live slug exists; else unique live project whose `repo_path` is an ancestor of `cwd` (longest `repo_path` wins); else `project_required`
  - `tb project detect [--json]` prints the Project (human: `slug  name`)
  - `task create` / `task list`: `--project` optional; omitted calls `detect_project(cwd, TASKBOARD_PROJECT)`
  - `tb inbox` unchanged

- [x] **Step 1: Write the failing tests**

`crates/application/tests/detect.rs` uses the same `test_app` / `cli_actor` pattern as `crates/application/tests/inbox.rs`, with the five cases from Task 9: env slug, unique ancestor, missing → `project_required`, unknown env → `not_found`, longest path wins.

Add `project_required_code` to `crates/application/src/error.rs` tests: `assert_eq!(AppError::ProjectRequired.code(), "project_required");`

CLI in `cli_json.rs`: `project_detect_json_from_path`, `task_list_without_project_uses_detect`, `task_list_without_project_unlinked_is_project_required` as in Task 9.

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p taskboard-application --test detect -- --test-threads=1`

Expected: FAIL compile (`detect_project` / `ProjectRequired` missing).

- [ ] **Step 3: Implement**

`AppError::ProjectRequired` with `#[error("pass --project or set TASKBOARD_PROJECT")]`.

`detect.rs`:

```rust
pub fn is_repo_ancestor(repo_path: &str, cwd: &Path) -> bool {
    if repo_path.is_empty() { return false; }
    let repo = Path::new(repo_path);
    let cwd_resolved = cwd.canonicalize().unwrap_or_else(|_| cwd.to_path_buf());
    let repo_resolved = if repo.exists() {
        repo.canonicalize().unwrap_or_else(|_| repo.to_path_buf())
    } else {
        repo.to_path_buf()
    };
    let parent = repo_resolved.to_string_lossy();
    let child = cwd_resolved.to_string_lossy();
    if parent == child { return true; }
    let prefix = if parent.ends_with('/') { parent.to_string() } else { format!("{parent}/") };
    child.starts_with(&prefix)
}
```

`App::detect_project`: if `env_slug` is non-empty, find that live slug or `NotFound { entity: "project", id }`; else `pick_by_repo_path` (max `repo_path` len among ancestors) or `ProjectRequired`.

CLI: `ProjectCommand::Detect`. `TaskCommand::{Create,List}` `project: Option<String>`. Helper:

```rust
async fn resolve_project(app: &App, project: Option<String>) -> Result<String, AppError> {
    if let Some(slug) = project { return Ok(slug); }
    let cwd = std::env::current_dir().map_err(|e| AppError::Io(e.to_string()))?;
    let env_slug = std::env::var("TASKBOARD_PROJECT").ok();
    Ok(app.detect_project(&cwd, env_slug.as_deref().filter(|s| !s.is_empty())).await?.slug)
}
```

API `app_error` status: `ProjectRequired => BAD_REQUEST`.

AGENTS.md session workflow first two bullets:

```markdown
1. `tb project detect --json` — if `project_required`, `project list --json` then `project add --name taskboard --path <repo-root> --json`
2. `task list --project taskboard --json` — reuse a matching `TASK-n`, or `task create --project taskboard --title "<short title>" --json`
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p taskboard-application --test detect -- --test-threads=1 && cargo test -p taskboard-application error:: && cargo test -p taskboard-cli --test cli_json -- --test-threads=1`

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/application crates/cli crates/api AGENTS.md docs/superpowers/plans/2026-09-16-issue-55-project-detect.md
git commit -m "feat: detect current project from repo path and TASKBOARD_PROJECT"
```

---

## Self-review

1. **Spec coverage:** Resolution order, detect CLI, optional `--project` on create/list, inbox excluded, AGENTS.md, not_found vs project_required, longest path (§8).
2. **Placeholder scan:** No TBD. Tests copied from Task 9 with complete code.
3. **Type consistency:** `detect_project(&Path, Option<&str>) -> Result<Project, AppError>` used by CLI and tests.
