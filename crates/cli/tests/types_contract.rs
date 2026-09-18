use std::fs;
use std::path::PathBuf;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap()
}

fn read_repo(rel: &str) -> String {
    fs::read_to_string(repo_root().join(rel)).unwrap_or_else(|e| panic!("read {rel}: {e}"))
}

fn snake_to_camel(name: &str) -> String {
    let mut out = String::new();
    let mut upper = false;
    for ch in name.chars() {
        if ch == '_' {
            upper = true;
        } else if upper {
            out.extend(ch.to_uppercase());
            upper = false;
        } else {
            out.push(ch);
        }
    }
    out
}

fn rust_struct_fields(src: &str, name: &str) -> Vec<String> {
    let header = format!("pub struct {name} {{");
    let start = src
        .find(&header)
        .unwrap_or_else(|| panic!("missing rust struct {name}"));
    let body = &src[start + header.len()..];
    let end = body
        .find("\n}")
        .unwrap_or_else(|| panic!("unclosed {name}"));
    body[..end]
        .lines()
        .filter_map(|line| {
            let line = line.trim();
            let rest = line.strip_prefix("pub ")?;
            let field = rest.split(':').next()?.trim();
            if field.is_empty() || field.starts_with("//") {
                return None;
            }
            Some(field.to_string())
        })
        .collect()
}

fn rust_enum_variants(src: &str, name: &str) -> Vec<String> {
    let header = format!("pub enum {name} {{");
    let start = src
        .find(&header)
        .unwrap_or_else(|| panic!("missing rust enum {name}"));
    let body = &src[start + header.len()..];
    let end = body
        .find("\n}")
        .unwrap_or_else(|| panic!("unclosed {name}"));
    body[..end]
        .lines()
        .filter_map(|line| {
            let line = line.trim().trim_end_matches(',');
            if line.is_empty() || line.starts_with('#') || line.starts_with("//") {
                return None;
            }
            Some(line.to_string())
        })
        .collect()
}

fn ts_interface_fields(src: &str, name: &str) -> Vec<String> {
    let header = format!("export interface {name} {{");
    let start = src
        .find(&header)
        .unwrap_or_else(|| panic!("missing TS interface {name}"));
    let body = &src[start + header.len()..];
    let end = body
        .find("\n}")
        .unwrap_or_else(|| panic!("unclosed {name}"));
    body[..end]
        .lines()
        .filter_map(|line| {
            let line = line.trim();
            if line.is_empty() || line.starts_with('/') || line.starts_with('*') {
                return None;
            }
            let name = line.split(':').next()?.trim().trim_end_matches('?');
            if name.is_empty() {
                return None;
            }
            Some(name.to_string())
        })
        .collect()
}

fn ts_string_union(src: &str, name: &str) -> Vec<String> {
    let header = format!("export type {name} =");
    let start = src
        .find(&header)
        .unwrap_or_else(|| panic!("missing TS type {name}"));
    let line = src[start..]
        .lines()
        .next()
        .unwrap_or_else(|| panic!("empty TS type {name}"));
    line.split('=')
        .nth(1)
        .unwrap()
        .split('|')
        .map(|part| {
            part.trim()
                .trim_end_matches(';')
                .trim()
                .trim_matches('"')
                .to_string()
        })
        .filter(|part| !part.is_empty())
        .collect()
}

#[test]
fn task_summary_keys_match_types_package() {
    let rust = rust_struct_fields(&read_repo("crates/core/src/models.rs"), "TaskSummary");
    let ts = ts_interface_fields(&read_repo("packages/types/src/index.ts"), "TaskSummary");
    let rust_camel: Vec<String> = rust.iter().map(|f| snake_to_camel(f)).collect();
    assert_eq!(
        rust_camel, ts,
        "TaskSummary rust fields must map 1:1 to packages/types TaskSummary"
    );
}

#[test]
fn task_update_product_fields_are_on_task_patch() {
    let rust = rust_struct_fields(
        &read_repo("crates/application/src/commands.rs"),
        "TaskUpdate",
    );
    let product: Vec<String> = rust
        .into_iter()
        .filter(|f| f != "display_id" && f != "revision")
        .map(|f| snake_to_camel(&f))
        .collect();
    assert_eq!(product, ["title", "worktreePath", "branch"]);
    let ts = ts_interface_fields(&read_repo("packages/types/src/index.ts"), "TaskPatch");
    for field in &product {
        assert!(
            ts.contains(field),
            "TaskPatch missing CLI task update field {field}"
        );
    }
    let http = rust_struct_fields(&read_repo("crates/api/src/dto.rs"), "PatchTaskBody");
    let desktop = rust_struct_fields(
        &read_repo("crates/desktop-commands/src/commands.rs"),
        "TaskPatchArgs",
    );
    for field in ["title", "worktree_path", "branch"] {
        assert!(
            http.contains(&field.to_string()),
            "PatchTaskBody missing {field}"
        );
        assert!(
            desktop.contains(&field.to_string()),
            "TaskPatchArgs missing {field}"
        );
    }
}

#[test]
fn entity_type_includes_comment_and_check() {
    let rust = rust_enum_variants(&read_repo("crates/core/src/models.rs"), "EntityType");
    let ts = ts_string_union(&read_repo("packages/types/src/index.ts"), "EntityType");
    let rust_snake: Vec<String> = rust
        .iter()
        .map(|v| {
            let mut out = String::new();
            for (i, ch) in v.chars().enumerate() {
                if ch.is_uppercase() && i > 0 {
                    out.push('_');
                }
                out.extend(ch.to_lowercase());
            }
            out
        })
        .collect();
    assert_eq!(rust_snake, ts);
    assert!(ts.iter().any(|v| v == "comment"));
    assert!(ts.iter().any(|v| v == "check"));
}

#[test]
fn response_dtos_live_only_in_wire() {
    let names = [
        "ProjectDto",
        "TaskSummaryDto",
        "LinkDto",
        "CommentDto",
        "CheckDto",
        "RunDto",
        "ActivityDto",
        "TaskDetailDto",
        "SyncDeltaDto",
        "TrashDto",
        "InboxItemDto",
        "StatusLineDto",
        "InboxCountsDto",
        "BoardStatusDto",
        "OccupancyRunDto",
        "OccupancyGroupDto",
        "UiStateDto",
        "UndoResultDto",
    ];
    let wire = read_repo("crates/wire/src/lib.rs");
    let api = read_repo("crates/api/src/dto.rs");
    let desktop = read_repo("crates/desktop-commands/src/dto.rs");
    for name in names {
        let header = format!("pub struct {name} {{");
        assert!(
            wire.contains(&header),
            "{name} must be defined in crates/wire"
        );
        assert!(
            !api.contains(&header),
            "{name} must not be redefined in api dto.rs"
        );
        assert!(
            !desktop.contains(&header),
            "{name} must not be redefined in desktop-commands dto.rs"
        );
    }
    assert!(
        wire.contains("pub fn json_keys_to_camel"),
        "json_keys_to_camel must live in wire"
    );
    assert!(
        !api.contains("pub fn json_keys_to_camel"),
        "api must not redefine json_keys_to_camel"
    );
    assert!(
        !desktop.contains("pub fn json_keys_to_camel"),
        "desktop must not redefine json_keys_to_camel"
    );
}
