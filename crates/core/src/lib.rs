pub fn workspace_name() -> &'static str {
    "taskboard"
}

#[cfg(test)]
mod tests {
    #[test]
    fn workspace_name_is_taskboard() {
        assert_eq!(super::workspace_name(), "taskboard");
    }
}
