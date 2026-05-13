use std::path::Path;

/// Compute disambiguated display name for a path among all paths.
///
/// - If basename is unique among `all_paths`: return basename alone.
/// - If basename collides: return shortest distinguishing `parent/basename`.
///
/// Example: `/henull2/public` among `[/henull2/public, /henullcom/public]`
/// → returns `henull2/public`.
pub fn disambiguate_name(path: &str, all_paths: &[&str]) -> String {
    let basename = basename_from_path(path);
    let same_basename: Vec<&&str> = all_paths
        .iter()
        .filter(|p| basename_from_path(p) == basename)
        .collect();

    if same_basename.len() <= 1 {
        return basename;
    }

    let parts: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();
    let depth = parts.len();

    for take in 2..=depth {
        let start = depth.saturating_sub(take);
        let suffix = &parts[start..];

        let mut is_unique = true;
        for other in &same_basename {
            if **other == path {
                continue;
            }
            let other_parts: Vec<&str> = other.split('/').filter(|s| !s.is_empty()).collect();
            let other_start = other_parts.len().saturating_sub(take);
            if other_parts.len() >= take && &other_parts[other_start..] == suffix {
                is_unique = false;
                break;
            }
        }
        if is_unique {
            return suffix.join("/");
        }
    }

    // Fallback: full path
    parts.join("/")
}

pub fn basename_from_path(path: &str) -> String {
    Path::new(path)
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| path.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

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

    #[test]
    fn disambiguate_unique_basename() {
        assert_eq!(
            disambiguate_name("/a/public", &["/a/public", "/b/other"]),
            "public"
        );
    }

    #[test]
    fn disambiguate_collision_uses_parent() {
        assert_eq!(
            disambiguate_name("/henull2/public", &["/henull2/public", "/henullcom/public"]),
            "henull2/public"
        );
        assert_eq!(
            disambiguate_name(
                "/henullcom/public",
                &["/henull2/public", "/henullcom/public"]
            ),
            "henullcom/public"
        );
    }

    #[test]
    fn disambiguate_three_way_collision() {
        let paths = ["/a/bot", "/c/bot", "/e/bot"];
        assert_eq!(disambiguate_name("/a/bot", &paths), "a/bot");
        assert_eq!(disambiguate_name("/c/bot", &paths), "c/bot");
        assert_eq!(disambiguate_name("/e/bot", &paths), "e/bot");
    }

    #[test]
    fn disambiguate_parent_also_collides() {
        let paths = ["/x/a/bot", "/x/c/bot"];
        assert_eq!(disambiguate_name("/x/a/bot", &paths), "a/bot");
        assert_eq!(disambiguate_name("/x/c/bot", &paths), "c/bot");
    }

    #[test]
    fn disambiguate_single_path() {
        assert_eq!(
            disambiguate_name("/home/project", &["/home/project"]),
            "project"
        );
    }

    #[test]
    fn disambiguate_same_tail_different_grandparent() {
        // a/b/c vs d/b/c → need grandparent to disambiguate
        let paths = ["/a/b/c", "/d/b/c"];
        assert_eq!(disambiguate_name("/a/b/c", &paths), "a/b/c");
        assert_eq!(disambiguate_name("/d/b/c", &paths), "d/b/c");
    }

    #[test]
    fn disambiguate_identical_paths_returns_basename() {
        // Same path appearing twice → basename collision → uses parent/basename
        let paths = ["/work/project", "/work/project"];
        assert_eq!(disambiguate_name("/work/project", &paths), "work/project");
    }
}
