# Unify AppError → wire envelope (#155) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** One shared error field mapping (`code`, `message`, `field`, `current`, `slug`) used by CLI, HTTP, and desktop; HTTP keeps status-code selection.

**Architecture:** Add `WireError` in `taskboard-wire` built from `&AppError` (camelCase `current` via `json_keys_to_camel`, include `DuplicateSlug.slug`). Surfaces wrap it in their envelope shape only.

**Tech Stack:** Rust application + wire + api + desktop-commands + cli.

## Global Constraints

- Behavior-preserving aside from aligning missing fields (slug on all surfaces; camelCase `current` everywhere)
- HTTP status table stays in `api`
- No new error codes

User already chose sequential inline execution.

## File map

- Modify: `crates/wire/src/lib.rs` — `WireError` + `From<&AppError>`
- Modify: `crates/api/src/routes.rs` — `app_error` uses WireError
- Modify: `crates/desktop-commands/src/error.rs` — From via WireError
- Modify: `crates/cli/src/output.rs` — `error_value` uses WireError
- Test: wire unit test + existing desktop/cli/api tests

---

### Task 1: WireError

```rust
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WireError {
    pub code: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub field: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub current: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub slug: Option<String>,
}

impl From<&AppError> for WireError { /* match + json_keys_to_camel on current */ }
impl From<AppError> for WireError { fn from(e: AppError) -> Self { WireError::from(&e) } }
```

Desktop `AppErrorDto` can become a type alias or thin wrapper copying WireError fields (keep `current` always present as Option for existing shape — desktop serializes null current today). Prefer mapping WireError → AppErrorDto preserving Option fields without skip_serializing_if if tests expect null.

- [ ] Implement + unit test DuplicateSlug includes slug; RevisionConflict camelCases current
- [ ] Wire CLI/HTTP/desktop
- [ ] Tests + PR

## Self-review

Acceptance: one mapping, shared fields including slug, HTTP status local — covered.
