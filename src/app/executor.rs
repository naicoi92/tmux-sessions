use crate::adapters::tmux::TmuxSource;
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
    let session_name = sanitize_session_name(&extract_session_name(path));

    if tmux.has_session(&session_name).unwrap_or(false) {
        let target = tmux.new_window(&session_name, path)?;
        tmux.select_window(&target)?;
        tmux.switch_client(&session_name)?;
        Ok(ExitReason::SwitchTo(target))
    } else {
        tmux.new_session(&session_name, path)?;
        tmux.switch_client(&session_name)?;
        Ok(ExitReason::SwitchTo(session_name))
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

/// Resolve collision: `base`, `base-1`, `base-2`, ...
pub fn resolve_session_name(base: &str, existing: &[String]) -> String {
    if !existing.iter().any(|s| s == base) {
        return base.to_string();
    }

    let mut suffix: u32 = 1;
    loop {
        let candidate = format!("{base}-{suffix}");
        if !existing.iter().any(|s| s == &candidate) {
            return candidate;
        }
        suffix += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapters::tmux::FakeTmuxSource;

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
        tmux.existing_sessions = vec!["myproject".into()];
        tmux.current_session_name = "other-session".into();

        let action =
            Action::goto_zoxide("/home/user/myproject".into(), "/home/user/myproject".into());
        let result = ActionExecutor::execute(&action, &tmux).unwrap();

        assert_eq!(result, ExitReason::SwitchTo("myproject:99".into()));
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
        tmux.existing_sessions = vec!["_dotfiles".into()];
        tmux.current_session_name = "current".into();

        let action =
            Action::goto_zoxide("/home/user/.dotfiles".into(), "/home/user/.dotfiles".into());
        let result = ActionExecutor::execute(&action, &tmux).unwrap();

        assert_eq!(result, ExitReason::SwitchTo("_dotfiles:99".into()));
    }
}
