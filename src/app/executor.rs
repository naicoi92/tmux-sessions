use crate::adapters::tmux::{RawWindow, TmuxSource};
use crate::domain::action::Action;
use crate::domain::entry::EntryType;
use crate::domain::error::ActionError;
use crate::domain::path_name::basename_from_path;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExitReason {
    Quit,
    SwitchTo(String),
    Reload,
}

pub struct ActionExecutor;

impl ActionExecutor {
    pub fn execute(action: &Action, tmux: &dyn TmuxSource) -> Result<ExitReason, ActionError> {
        match action {
            Action::Goto {
                target,
                path,
                entry_type,
            } => execute_goto(target, path, *entry_type, tmux),
            Action::Kill { target, entry_type } => execute_kill(target, *entry_type, tmux),
            Action::TogglePreview | Action::Reload | Action::Quit => Ok(ExitReason::Quit),
        }
    }
}

fn execute_goto(
    target: &str,
    path: &str,
    entry_type: EntryType,
    tmux: &dyn TmuxSource,
) -> Result<ExitReason, ActionError> {
    match entry_type {
        EntryType::Window => execute_window_goto(target, tmux),
        EntryType::Zoxide => execute_zoxide_goto(path, tmux),
    }
}

fn execute_window_goto(target: &str, tmux: &dyn TmuxSource) -> Result<ExitReason, ActionError> {
    tmux.select_window(target)?;
    if let Some(session) = extract_session_from_target(target) {
        tmux.switch_client(&session)?;
    }
    Ok(ExitReason::SwitchTo(target.to_string()))
}

fn execute_zoxide_goto(path: &str, tmux: &dyn TmuxSource) -> Result<ExitReason, ActionError> {
    let windows = tmux.list_windows().unwrap_or_default();
    let sessions = tmux.list_sessions().unwrap_or_default();
    let existing_names: Vec<String> = sessions.iter().map(|s| s.session_name.clone()).collect();

    match resolve_zoxide_action(path, &windows, &existing_names) {
        ZoxideAction::SwitchToExisting { target } => execute_window_goto(&target, tmux),
        ZoxideAction::CreateNewSession { name } => {
            tmux.new_session(&name, path)?;
            tmux.switch_client(&name)?;
            Ok(ExitReason::SwitchTo(name))
        }
    }
}

fn execute_kill(
    target: &str,
    entry_type: EntryType,
    tmux: &dyn TmuxSource,
) -> Result<ExitReason, ActionError> {
    match entry_type {
        EntryType::Window => {
            tmux.kill_window(target)?;
        }
        EntryType::Zoxide => {
            let session_name = extract_session_name(target);
            tmux.kill_session(&session_name)?;
        }
    }
    Ok(ExitReason::Reload)
}

pub fn extract_session_name(path: &str) -> String {
    basename_from_path(path)
}

fn extract_session_from_target(target: &str) -> Option<String> {
    target
        .split_once(':')
        .map(|(session, _)| session.to_string())
}

/// Ký tự hợp lệ cho tmux session name: alphanumeric, `-`, `_`, `.`.
fn is_valid_session_char(c: char) -> bool {
    c.is_alphanumeric() || c == '-' || c == '_' || c == '.'
}

/// Sanitize basename thành tmux session name.
/// Thay ký tự invalid bằng `_`, prefix `.` bằng `_`, rỗng → `"_"`.
pub fn sanitize_session_name(basename: &str) -> String {
    let trimmed = basename.trim();
    if trimmed.is_empty() {
        return "_".to_string();
    }

    let mut result = String::with_capacity(trimmed.len());
    let mut chars = trimmed.chars();

    // Ký tự đầu: `.` bị cấm vì tmux hiểu là socket path
    if let Some(c) = chars.next() {
        if is_valid_session_char(c) && c != '.' {
            result.push(c);
        } else {
            result.push('_');
        }
    }

    for c in chars {
        if is_valid_session_char(c) {
            result.push(c);
        } else {
            result.push('_');
        }
    }

    if result.is_empty() {
        "_".to_string()
    } else {
        result
    }
}

/// Decision for zoxide goto: switch to existing window or create new session.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ZoxideAction {
    /// An existing tmux window already works at this path — switch to it.
    SwitchToExisting { target: String },
    /// No existing window matches — create a new session.
    CreateNewSession { name: String },
}

/// Compare two filesystem paths for equality.
/// Direct string comparison first; falls back to `canonicalize` for symlinks.
fn paths_match(a: &str, b: &str) -> bool {
    if a == b {
        return true;
    }
    match (std::fs::canonicalize(a), std::fs::canonicalize(b)) {
        (Ok(ca), Ok(cb)) => ca == cb,
        _ => false,
    }
}

/// Resolve what action to take when user selects a zoxide directory.
///
/// 1. If any existing tmux window's `pane_current_path` matches `path`, switch to it.
/// 2. Otherwise, create a new session with a collision-safe name.
pub fn resolve_zoxide_action(
    path: &str,
    windows: &[RawWindow],
    existing_session_names: &[String],
) -> ZoxideAction {
    for w in windows {
        if paths_match(&w.window_path, path) {
            return ZoxideAction::SwitchToExisting {
                target: format!("{}:{}", w.session_name, w.window_index),
            };
        }
    }

    let base = sanitize_session_name(&extract_session_name(path));
    let name = resolve_session_name(&base, existing_session_names);
    ZoxideAction::CreateNewSession { name }
}

/// Resolve collision-safe tmux session name from a path.
/// Uses basename + numeric suffix: `public`, `public-1`, `public-2`.
pub fn resolve_session_name(path: &str, existing: &[String]) -> String {
    let basename = sanitize_session_name(&basename_from_path(path));
    if !existing.iter().any(|s| s == &basename) {
        return basename;
    }

    let mut suffix: u32 = 1;
    loop {
        let candidate = format!("{basename}-{suffix}");
        if !existing.iter().any(|s| s == &candidate) {
            return candidate;
        }
        suffix += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapters::tmux::{FakeTmuxSource, RawSession, RawWindow};

    #[test]
    fn extract_basename() {
        assert_eq!(extract_session_name("/home/user/myproject"), "myproject");
    }

    #[test]
    fn extract_nested_path() {
        assert_eq!(extract_session_name("/a/b/c/deep-project"), "deep-project");
    }

    #[test]
    fn extract_bare_name() {
        assert_eq!(extract_session_name("myproject"), "myproject");
    }

    #[test]
    fn extract_root_path() {
        assert_eq!(extract_session_name("/"), "/");
    }

    #[test]
    fn extract_trailing_slash() {
        assert_eq!(extract_session_name("/home/user/proj/"), "proj");
    }

    #[test]
    fn sanitize_normal_name() {
        assert_eq!(sanitize_session_name("myproject"), "myproject");
    }

    #[test]
    fn sanitize_with_dashes() {
        assert_eq!(sanitize_session_name("my-project_v2"), "my-project_v2");
    }

    #[test]
    fn sanitize_dot_prefix_replaced() {
        assert_eq!(sanitize_session_name(".dotfiles"), "_dotfiles");
    }

    #[test]
    fn sanitize_dot_middle_kept() {
        assert_eq!(sanitize_session_name("config.v2"), "config.v2");
    }

    #[test]
    fn sanitize_special_chars_replaced() {
        assert_eq!(sanitize_session_name("my project@2024"), "my_project_2024");
    }

    #[test]
    fn sanitize_colon_replaced() {
        assert_eq!(sanitize_session_name("session:name"), "session_name");
    }

    #[test]
    fn sanitize_empty_string() {
        assert_eq!(sanitize_session_name(""), "_");
    }

    #[test]
    fn sanitize_whitespace_only() {
        assert_eq!(sanitize_session_name("   "), "_");
    }

    #[test]
    fn sanitize_whitespace_trimmed() {
        assert_eq!(sanitize_session_name("  project  "), "project");
    }

    #[test]
    fn sanitize_all_invalid() {
        assert_eq!(sanitize_session_name("@#$%"), "____");
    }

    #[test]
    fn sanitize_root_slash() {
        // "/" is now valid in tmux session names
        assert_eq!(sanitize_session_name("/"), "_");
    }

    #[test]
    fn resolve_no_collision() {
        let existing: Vec<String> = vec!["other".into()];
        assert_eq!(resolve_session_name("project", &existing), "project");
    }

    #[test]
    fn resolve_single_collision() {
        let existing: Vec<String> = vec!["project".into()];
        assert_eq!(resolve_session_name("project", &existing), "project-1");
    }

    #[test]
    fn resolve_multiple_collisions() {
        let existing: Vec<String> = vec!["project".into(), "project-1".into()];
        assert_eq!(resolve_session_name("project", &existing), "project-2");
    }

    #[test]
    fn resolve_gap_in_suffixes() {
        let existing: Vec<String> = vec!["project".into(), "project-2".into()];
        assert_eq!(resolve_session_name("project", &existing), "project-1");
    }

    #[test]
    fn resolve_empty_existing() {
        let existing: Vec<String> = vec![];
        assert_eq!(resolve_session_name("project", &existing), "project");
    }

    #[test]
    fn zoxide_goto_existing_session_creates_window_in_that_session() {
        let mut tmux = FakeTmuxSource::new();
        // Existing session "myproject" with a window at the same path
        tmux.sessions = vec![RawSession {
            session_name: "myproject".into(),
            attached: false,
            session_activity: None,
        }];
        tmux.windows = vec![RawWindow {
            session_name: "myproject".into(),
            window_index: "0".into(),
            window_name: "work".into(),
            window_path: "/home/user/myproject".into(),
            window_activity: None,
        }];
        tmux.existing_sessions = vec!["myproject".into()];
        tmux.current_session_name = "other-session".into();

        let action =
            Action::goto_zoxide("/home/user/myproject".into(), "/home/user/myproject".into());
        let result = ActionExecutor::execute(&action, &tmux).unwrap();

        // Path matches existing window → switch to it
        assert_eq!(result, ExitReason::SwitchTo("myproject:0".into()));
    }

    #[test]
    fn zoxide_goto_no_session_creates_new_session() {
        let mut tmux = FakeTmuxSource::new();
        tmux.current_session_name = "current".into();

        let action = Action::goto_zoxide(
            "/home/user/newproject".into(),
            "/home/user/newproject".into(),
        );
        let result = ActionExecutor::execute(&action, &tmux).unwrap();

        assert_eq!(result, ExitReason::SwitchTo("newproject".into()));
    }

    #[test]
    fn zoxide_goto_sanitizes_session_name() {
        let mut tmux = FakeTmuxSource::new();
        // Session "_dotfiles" exists but no window at that path → collision-safe name
        tmux.sessions = vec![RawSession {
            session_name: "_dotfiles".into(),
            attached: false,
            session_activity: None,
        }];
        tmux.existing_sessions = vec!["_dotfiles".into()];
        tmux.current_session_name = "current".into();

        let action =
            Action::goto_zoxide("/home/user/.dotfiles".into(), "/home/user/.dotfiles".into());
        let result = ActionExecutor::execute(&action, &tmux).unwrap();

        assert_eq!(result, ExitReason::SwitchTo("_dotfiles-1".into()));
    }
}
