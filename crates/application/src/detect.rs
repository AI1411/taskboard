use std::path::Path;

use taskboard_core::Project;

pub fn is_repo_ancestor(repo_path: &str, cwd: &Path) -> bool {
    if repo_path.is_empty() {
        return false;
    }
    let repo = Path::new(repo_path);
    let cwd_resolved = cwd.canonicalize().unwrap_or_else(|_| cwd.to_path_buf());
    let repo_resolved = if repo.exists() {
        repo.canonicalize().unwrap_or_else(|_| repo.to_path_buf())
    } else {
        repo.to_path_buf()
    };
    let parent = repo_resolved.to_string_lossy();
    let child = cwd_resolved.to_string_lossy();
    if parent == child {
        return true;
    }
    let prefix = if parent.ends_with('/') {
        parent.to_string()
    } else {
        format!("{parent}/")
    };
    child.starts_with(&prefix)
}

pub fn pick_by_repo_path(projects: &[Project], cwd: &Path) -> Option<Project> {
    projects
        .iter()
        .filter(|project| {
            project
                .repo_path
                .as_deref()
                .is_some_and(|path| is_repo_ancestor(path, cwd))
        })
        .max_by_key(|project| {
            project
                .repo_path
                .as_ref()
                .map(|path| path.len())
                .unwrap_or(0)
        })
        .cloned()
}

#[cfg(test)]
mod tests {
    use super::is_repo_ancestor;
    use std::path::Path;

    #[test]
    fn prefix_does_not_match_sibling_name() {
        assert!(is_repo_ancestor(
            "/tmp/renai",
            Path::new("/tmp/renai/crates")
        ));
        assert!(!is_repo_ancestor(
            "/tmp/renai",
            Path::new("/tmp/renai-other")
        ));
    }
}
