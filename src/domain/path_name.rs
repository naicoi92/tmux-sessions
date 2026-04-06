use std::path::Path;

pub fn basename_from_path(path: &str) -> String {
    Path::new(path)
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| path.to_string())
}

#[cfg(test)]
mod tests {
    use super::basename_from_path;

    #[test]
    fn basename_extracts_normal_path() {
        assert_eq!(basename_from_path("/home/user/myproject"), "myproject");
    }

    #[test]
    fn basename_keeps_root_fallback() {
        assert_eq!(basename_from_path("/"), "/");
    }

    #[test]
    fn basename_handles_trailing_slash() {
        assert_eq!(basename_from_path("/home/user/proj/"), "proj");
    }
}
