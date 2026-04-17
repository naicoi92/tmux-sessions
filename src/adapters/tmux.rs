mod adapter_impl;
mod capture;
mod command;
mod fake;
mod parser;
mod raw;

use crate::domain::error::{ActionError, AdapterError};

pub use adapter_impl::TmuxAdapter;
pub use fake::{FakeTmuxCall, FakeTmuxSource};
pub use raw::{RawSession, RawWindow};

pub trait TmuxSource {
    fn list_windows(&self) -> Result<Vec<RawWindow>, AdapterError>;
    fn list_sessions(&self) -> Result<Vec<RawSession>, AdapterError>;
    fn current_session(&self) -> Result<String, AdapterError>;
    fn current_window_index(&self) -> Result<String, AdapterError>;
    fn has_session(&self, name: &str) -> Result<bool, AdapterError>;
    fn select_window(&self, target: &str) -> Result<(), ActionError>;
    fn new_session(&self, name: &str, path: &str) -> Result<(), ActionError>;
    fn new_window(&self, session: &str, path: &str) -> Result<String, ActionError>;
    fn switch_client(&self, target: &str) -> Result<(), ActionError>;
    fn kill_window(&self, target: &str) -> Result<(), ActionError>;
    fn kill_session(&self, name: &str) -> Result<(), ActionError>;
    fn capture_pane(&self, target: &str, line_count: usize) -> Result<String, AdapterError>;
    fn capture_pane_with_size(
        &self,
        target: &str,
        line_count: usize,
        _width: Option<u16>,
        height: Option<u16>,
    ) -> Result<String, AdapterError> {
        let effective_line_count = height.map(|h| h as usize).unwrap_or(line_count);
        capture::capture_best_effort(target, effective_line_count)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::tmux_window_mapper::map_raw_windows_to_entries;
    use crate::domain::entry::SortPriority;
    use std::os::unix::process::ExitStatusExt;
    use std::process::{ExitStatus, Output};

    fn make_output(code: i32, stdout: &str, stderr: &str) -> Output {
        Output {
            status: ExitStatus::from_raw(code),
            stdout: stdout.as_bytes().to_vec(),
            stderr: stderr.as_bytes().to_vec(),
        }
    }

    #[test]
    fn parse_windows_valid_input() {
        let input =
            "session1\t0\tmain\t/home/user\t1714000000\nsession1\t1\tother\t/home/user\t1714000100";
        let windows = parser::parse_windows(input).unwrap();
        assert_eq!(windows.len(), 2);
        assert_eq!(windows[0].session_name, "session1");
        assert_eq!(windows[0].window_index, "0");
        assert_eq!(windows[0].window_name, "main");
        assert_eq!(windows[0].window_activity, Some(1714000000));
        assert_eq!(windows[1].window_name, "other");
    }

    #[test]
    fn parse_windows_invalid_field_count() {
        let input = "session1\t0\tmain";
        let result = parser::parse_windows(input);
        assert!(result.is_err());
        match result.unwrap_err() {
            AdapterError::TmuxParse { detail, .. } => {
                assert!(detail.contains("got 3"));
            }
            _ => panic!("expected TmuxParse"),
        }
    }

    #[test]
    fn parse_sessions_valid_input() {
        let input = "s1\t1\ns2\t0";
        let sessions = parser::parse_sessions(input).unwrap();
        assert_eq!(sessions.len(), 2);
        assert!(sessions[0].attached);
        assert!(!sessions[1].attached);
    }

    #[test]
    fn windows_to_entries_marks_current() {
        let raw = vec![
            RawWindow {
                session_name: "s1".into(),
                window_index: "0".into(),
                window_name: "main".into(),
                window_path: "/home".into(),
                window_activity: None,
            },
            RawWindow {
                session_name: "s1".into(),
                window_index: "1".into(),
                window_name: "other".into(),
                window_path: "/home".into(),
                window_activity: None,
            },
        ];
        let entries = map_raw_windows_to_entries(raw, "s1", "0");
        assert_eq!(entries.len(), 2);
        assert!(entries[0].is_current);
        assert!(!entries[1].is_current);
        assert_eq!(entries[0].priority, SortPriority::CurrentWindow);
    }

    #[test]
    fn fake_source_returns_configured_windows() {
        let fake = FakeTmuxSource::with_window("test", "0", "main", "/path");
        let windows = fake.list_windows().unwrap();
        assert_eq!(windows.len(), 1);
        assert_eq!(windows[0].session_name, "test");
    }

    #[test]
    fn fake_source_has_session_check() {
        let fake = FakeTmuxSource::new();
        assert!(!fake.has_session("missing").unwrap());
    }

    #[test]
    fn fake_source_with_window_has_session() {
        let fake = FakeTmuxSource::with_window("test", "0", "main", "/path");
        assert!(fake.has_session("test").unwrap());
    }

    #[test]
    fn fake_source_capture_pane_returns_explicit_error() {
        let fake = FakeTmuxSource::new();

        let err = fake
            .capture_pane("test:0", 12)
            .expect_err("fake capture should fail to trigger fallback");

        match err {
            AdapterError::TmuxCommand { command, detail } => {
                assert!(command.contains("capture-pane"));
                assert!(detail.contains("fake capture unavailable"));
            }
            other => panic!("expected TmuxCommand error, got {other:?}"),
        }
    }

    #[test]
    fn fake_source_capture_pane_with_size_returns_explicit_error() {
        let fake = FakeTmuxSource::new();

        let err = fake
            .capture_pane_with_size("test:0", 12, Some(120), Some(20))
            .expect_err("sized fake capture should fail to trigger fallback");

        match err {
            AdapterError::TmuxCommand { command, detail } => {
                assert!(command.contains("capture-pane"));
                assert!(detail.contains("fake capture unavailable"));
            }
            other => panic!("expected TmuxCommand error, got {other:?}"),
        }
    }

    #[test]
    fn handle_tmux_output_returns_stdout_when_status_success() {
        let output = make_output(0, "hello\n", "");

        let result =
            command::handle_tmux_output("display-message -p '#{session_name}'", output).unwrap();

        assert_eq!(result, "hello");
    }

    #[test]
    fn handle_tmux_output_returns_tmux_command_error_when_status_non_zero() {
        let output = make_output(256, "", "no server running\n");

        let err = command::handle_tmux_output("capture-pane -t test:0", output)
            .expect_err("non-zero exit status must return AdapterError::TmuxCommand");

        match err {
            AdapterError::TmuxCommand { command, detail } => {
                assert_eq!(command, "capture-pane -t test:0");
                assert_eq!(detail, "no server running");
            }
            other => panic!("expected TmuxCommand error, got {other:?}"),
        }
    }

    #[test]
    fn handle_tmux_capture_output_preserves_whitespace_and_newlines() {
        let output = make_output(0, "  a\n  b\n\n", "");

        let result = command::handle_tmux_capture_output("capture-pane -t test:0", output).unwrap();

        assert_eq!(result, "  a\n  b\n\n");
    }

    #[test]
    fn capture_pane_args_include_alternate_screen_and_split_start_flag() {
        let args = capture::capture_pane_args("test:0", 12, true);

        assert_eq!(
            args,
            vec![
                "capture-pane".to_string(),
                "-t".to_string(),
                "test:0".to_string(),
                "-p".to_string(),
                "-e".to_string(),
                "-a".to_string(),
                "-S".to_string(),
                "-12".to_string(),
                "-N".to_string(),
            ]
        );
    }

    #[test]
    fn capture_pane_args_primary_mode_excludes_alternate_screen_flag() {
        let args = capture::capture_pane_args("test:0", 12, false);

        assert_eq!(
            args,
            vec![
                "capture-pane".to_string(),
                "-t".to_string(),
                "test:0".to_string(),
                "-p".to_string(),
                "-e".to_string(),
                "-S".to_string(),
                "-12".to_string(),
                "-N".to_string(),
            ]
        );
    }

    #[test]
    fn non_alternate_error_is_not_retried_as_primary_capture() {
        let detail = "can't find pane".to_string();
        let is_alt_miss = detail.to_ascii_lowercase().contains("no alternate screen");
        assert!(!is_alt_miss);
    }

    #[test]
    fn select_capture_content_prefers_primary_when_available() {
        let primary = Ok("primary screen".to_string());
        let alternate = Ok("alternate screen".to_string());

        let result = capture::select_capture_content(primary, alternate).unwrap();
        assert_eq!(result, "primary screen");
    }

    #[test]
    fn select_capture_content_uses_alternate_when_primary_empty() {
        let primary = Ok(String::new());
        let alternate = Ok("alternate screen".to_string());

        let result = capture::select_capture_content(primary, alternate).unwrap();
        assert_eq!(result, "alternate screen");
    }

    #[test]
    fn select_capture_content_falls_back_to_alternate_if_primary_errors() {
        let primary = Err(AdapterError::TmuxCommand {
            command: "capture-pane primary".to_string(),
            detail: "can't find pane".to_string(),
        });
        let alternate = Ok("alternate screen".to_string());

        let result = capture::select_capture_content(primary, alternate).unwrap();
        assert_eq!(result, "alternate screen");
    }

    #[test]
    fn select_capture_content_prefers_alternate_when_it_has_more_visible_content() {
        let primary = Ok("$ opencode".to_string());
        let alternate =
            Ok("████ panel\nline two with content\nline three with more content".to_string());

        let result = capture::select_capture_content(primary, alternate).unwrap();
        assert_eq!(
            result,
            "████ panel\nline two with content\nline three with more content"
        );
    }

    #[test]
    fn visible_content_score_ignores_ansi_escape_sequences() {
        let score = capture::visible_content_score("\x1b[31mhello\x1b[0m\n\x1b[2K");
        assert_eq!(score, 5);
    }

    #[test]
    fn should_prefer_alternate_returns_false_for_two_substantive_screens() {
        assert!(!capture::should_prefer_alternate(14, 1, 16, 1));
    }

    #[test]
    fn select_capture_content_prefers_primary_for_substantive_shell_output() {
        let primary = Ok("user@host:~$ ls\nfile1 file2\nuser@host:~$".to_string());
        let alternate = Ok("████ stale alt panel\nline 2\nline 3\nline 4".to_string());

        let result = capture::select_capture_content(primary, alternate).unwrap();
        assert_eq!(result, "user@host:~$ ls\nfile1 file2\nuser@host:~$");
    }
}
