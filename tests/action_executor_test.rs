use tmux_sessions::adapters::tmux::{FakeTmuxCall, FakeTmuxSource};
use tmux_sessions::app::executor::{
    extract_session_name, resolve_session_name, sanitize_session_name, ActionExecutor, ExitReason,
};
use tmux_sessions::domain::action::Action;
use tmux_sessions::domain::entry::EntryType;
use tmux_sessions::domain::error::{ActionError, AdapterError};

use tmux_sessions::adapters::tmux::{RawSession, RawWindow, TmuxSource};

use std::cell::RefCell;

fn fake_no_sessions() -> FakeTmuxSource {
    FakeTmuxSource {
        windows: vec![],
        sessions: vec![],
        current_session_name: "default".into(),
        current_window_idx: "0".into(),
        existing_sessions: vec![],
        fail_on: vec![],
    }
}

fn fake_with_session(name: &str) -> FakeTmuxSource {
    use tmux_sessions::adapters::tmux::RawSession;
    FakeTmuxSource {
        windows: vec![],
        sessions: vec![RawSession {
            session_name: name.into(),
            attached: true,
            session_activity: None,
        }],
        current_session_name: name.into(),
        current_window_idx: "0".into(),
        existing_sessions: vec![name.into()],
        fail_on: vec![],
    }
}

fn fake_with_failure(call: FakeTmuxCall) -> FakeTmuxSource {
    FakeTmuxSource {
        windows: vec![],
        sessions: vec![],
        current_session_name: "s".into(),
        current_window_idx: "0".into(),
        existing_sessions: vec![],
        fail_on: vec![call],
    }
}

#[derive(Default)]
struct RecordingTmuxSource {
    calls: RefCell<Vec<String>>,
    fail_switch_on: Option<String>,
}

impl RecordingTmuxSource {
    fn with_switch_failure(target: &str) -> Self {
        Self {
            calls: RefCell::new(vec![]),
            fail_switch_on: Some(target.to_string()),
        }
    }

    fn calls(&self) -> Vec<String> {
        self.calls.borrow().clone()
    }
}

impl TmuxSource for RecordingTmuxSource {
    fn list_windows(&self) -> Result<Vec<RawWindow>, AdapterError> {
        Ok(vec![])
    }

    fn list_sessions(&self) -> Result<Vec<RawSession>, AdapterError> {
        Ok(vec![])
    }

    fn current_session(&self) -> Result<String, AdapterError> {
        Ok("default".into())
    }

    fn current_window_index(&self) -> Result<String, AdapterError> {
        Ok("0".into())
    }

    fn has_session(&self, _name: &str) -> Result<bool, AdapterError> {
        Ok(false)
    }

    fn select_window(&self, target: &str) -> Result<(), ActionError> {
        self.calls
            .borrow_mut()
            .push(format!("select_window:{target}"));
        Ok(())
    }

    fn new_session(&self, _name: &str, _path: &str) -> Result<(), ActionError> {
        Ok(())
    }

    fn new_window(&self, session: &str, _path: &str) -> Result<String, ActionError> {
        Ok(format!("{session}:99"))
    }

    fn switch_client(&self, target: &str) -> Result<(), ActionError> {
        self.calls
            .borrow_mut()
            .push(format!("switch_client:{target}"));

        if self
            .fail_switch_on
            .as_ref()
            .is_some_and(|expected| expected == target)
        {
            return Err(ActionError::GotoFailed {
                target: target.to_string(),
                detail: "fake switch failure".to_string(),
            });
        }
        Ok(())
    }

    fn kill_window(&self, target: &str) -> Result<(), ActionError> {
        self.calls
            .borrow_mut()
            .push(format!("kill_window:{target}"));
        Ok(())
    }

    fn kill_session(&self, name: &str) -> Result<(), ActionError> {
        self.calls.borrow_mut().push(format!("kill_session:{name}"));
        Ok(())
    }

    fn capture_pane(&self, target: &str, _line_count: usize) -> Result<String, AdapterError> {
        Err(AdapterError::TmuxCommand {
            command: format!("capture-pane -t {target}"),
            detail: "not used in tests".to_string(),
        })
    }
}

#[test]
fn goto_window_returns_switch_to() {
    let fake = fake_no_sessions();
    let action = Action::goto_window("s1:0".into(), "/path".into());
    let result = ActionExecutor::execute(&action, &fake).unwrap();

    assert_eq!(result, ExitReason::SwitchTo("s1:0".into()));
}

#[test]
fn goto_zoxide_new_session_returns_switch_to() {
    let fake = fake_no_sessions();
    let action = Action::goto_zoxide("/home/user/project".into(), "/home/user/project".into());
    let result = ActionExecutor::execute(&action, &fake).unwrap();

    assert_eq!(result, ExitReason::SwitchTo("project".into()));
}

#[test]
fn goto_zoxide_existing_session_avoids_collision() {
    let fake = fake_with_session("myproject");
    let action = Action::goto_zoxide("/home/user/myproject".into(), "/home/user/myproject".into());
    let result = ActionExecutor::execute(&action, &fake).unwrap();

    assert_eq!(result, ExitReason::SwitchTo("myproject:99".into()));
}

#[test]
fn goto_window_selects_then_switches_client_for_session_target() {
    let tmux = RecordingTmuxSource::default();
    let action = Action::goto_window("work:3".into(), "/path".into());

    let result = ActionExecutor::execute(&action, &tmux).unwrap();

    assert_eq!(result, ExitReason::SwitchTo("work:3".into()));
    assert_eq!(
        tmux.calls(),
        vec![
            "select_window:work:3".to_string(),
            "switch_client:work".to_string()
        ]
    );
}

#[test]
fn goto_window_without_session_separator_does_not_switch_client() {
    let tmux = RecordingTmuxSource::default();
    let action = Action::goto_window("work".into(), "/path".into());

    let result = ActionExecutor::execute(&action, &tmux).unwrap();

    assert_eq!(result, ExitReason::SwitchTo("work".into()));
    assert_eq!(tmux.calls(), vec!["select_window:work".to_string()]);
}

#[test]
fn goto_window_switch_client_failure_happens_after_select_window() {
    let tmux = RecordingTmuxSource::with_switch_failure("work");
    let action = Action::goto_window("work:9".into(), "/path".into());

    let result = ActionExecutor::execute(&action, &tmux);

    assert!(result.is_err());
    assert_eq!(
        tmux.calls(),
        vec![
            "select_window:work:9".to_string(),
            "switch_client:work".to_string()
        ]
    );
}

#[test]
fn kill_window_returns_reload() {
    let fake = fake_no_sessions();
    let action = Action::kill_window("s1:2".into());
    let result = ActionExecutor::execute(&action, &fake).unwrap();

    assert_eq!(result, ExitReason::Reload);
}

#[test]
fn kill_zoxide_returns_reload() {
    let fake = fake_with_session("project");
    let action = Action::kill_zoxide("project".into());
    let result = ActionExecutor::execute(&action, &fake).unwrap();

    assert_eq!(result, ExitReason::Reload);
}

#[test]
fn kill_zoxide_failure_returns_error() {
    let fake = fake_with_failure(FakeTmuxCall::KillSession("project".into()));
    let action = Action::kill_zoxide("project".into());
    let result = ActionExecutor::execute(&action, &fake);

    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(err.contains("project"));
}

#[test]
fn kill_zoxide_extracts_session_name_from_path_target() {
    let fake = fake_with_failure(FakeTmuxCall::KillSession("project".into()));
    let action = Action::Kill {
        target: "/home/user/project".into(),
        entry_type: EntryType::Zoxide,
    };
    let result = ActionExecutor::execute(&action, &fake);

    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(err.contains("project"));
}

#[test]
fn goto_window_failure_returns_error() {
    let fake = fake_with_failure(FakeTmuxCall::SelectWindow("s1:0".into()));
    let action = Action::goto_window("s1:0".into(), "/path".into());
    let result = ActionExecutor::execute(&action, &fake);

    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(err.contains("s1:0"));
}

#[test]
fn kill_window_failure_returns_error() {
    let fake = fake_with_failure(FakeTmuxCall::KillWindow("s1:0".into()));
    let action = Action::kill_window("s1:0".into());
    let result = ActionExecutor::execute(&action, &fake);

    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(err.contains("s1:0"));
}

#[test]
fn new_session_failure_returns_error() {
    let fake = FakeTmuxSource {
        windows: vec![],
        sessions: vec![],
        current_session_name: "s".into(),
        current_window_idx: "0".into(),
        existing_sessions: vec![],
        fail_on: vec![FakeTmuxCall::NewSession {
            name: "proj".into(),
            path: "/proj".into(),
        }],
    };
    let action = Action::goto_zoxide("/proj".into(), "/proj".into());
    let result = ActionExecutor::execute(&action, &fake);

    assert!(result.is_err());
}

#[test]
fn switch_client_failure_returns_error() {
    let fake = fake_with_session("existing");
    let fake = FakeTmuxSource {
        fail_on: vec![FakeTmuxCall::SwitchClient("s1".into())],
        ..fake
    };
    let action = Action::goto_window("s1:0".into(), "/existing".into());
    let result = ActionExecutor::execute(&action, &fake);

    assert!(result.is_err());
}

#[test]
fn session_name_from_nested_path() {
    let fake = fake_no_sessions();
    let action = Action::goto_zoxide(
        "/a/deep/nested/project".into(),
        "/a/deep/nested/project".into(),
    );
    let result = ActionExecutor::execute(&action, &fake).unwrap();
    assert_eq!(result, ExitReason::SwitchTo("project".into()));
}

#[test]
fn session_name_from_simple_path() {
    let fake = fake_no_sessions();
    let action = Action::goto_zoxide("/home/user/dotfiles".into(), "/home/user/dotfiles".into());
    let result = ActionExecutor::execute(&action, &fake).unwrap();
    assert_eq!(result, ExitReason::SwitchTo("dotfiles".into()));
}

#[test]
fn kill_window_target_with_special_chars() {
    let fake = fake_no_sessions();
    let action = Action::kill_window("s1:0".into());
    let result = ActionExecutor::execute(&action, &fake).unwrap();
    assert_eq!(result, ExitReason::Reload);
}

#[test]
fn kill_zoxide_target_with_dashes() {
    let fake = fake_with_session("my-project");
    let action = Action::kill_zoxide("my-project".into());
    let result = ActionExecutor::execute(&action, &fake).unwrap();
    assert_eq!(result, ExitReason::Reload);
}

#[test]
fn toggle_preview_action_returns_quit() {
    let fake = fake_no_sessions();
    let result = ActionExecutor::execute(&Action::TogglePreview, &fake).unwrap();
    assert_eq!(result, ExitReason::Quit);
}

#[test]
fn reload_action_returns_quit() {
    let fake = fake_no_sessions();
    let result = ActionExecutor::execute(&Action::Reload, &fake).unwrap();
    assert_eq!(result, ExitReason::Quit);
}

#[test]
fn quit_action_returns_quit() {
    let fake = fake_no_sessions();
    let result = ActionExecutor::execute(&Action::Quit, &fake).unwrap();
    assert_eq!(result, ExitReason::Quit);
}

#[test]
fn exit_reason_send_sync() {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<ExitReason>();
}

#[test]
fn goto_zoxide_invalid_basename_sanitized() {
    use tmux_sessions::adapters::tmux::RawSession;
    let fake = FakeTmuxSource {
        windows: vec![],
        sessions: vec![RawSession {
            session_name: "_".into(),
            attached: true,
            session_activity: None,
        }],
        current_session_name: "_".into(),
        current_window_idx: "0".into(),
        existing_sessions: vec!["_".into()],
        fail_on: vec![],
    };
    let action = Action::goto_zoxide("/".into(), "/".into());
    let result = ActionExecutor::execute(&action, &fake).unwrap();
    assert_eq!(result, ExitReason::SwitchTo("_:99".into()));
}

#[test]
fn goto_zoxide_dot_prefix_sanitized() {
    let fake = fake_no_sessions();
    let action = Action::goto_zoxide("/home/user/.dotfiles".into(), "/home/user/.dotfiles".into());
    let result = ActionExecutor::execute(&action, &fake).unwrap();
    assert_eq!(result, ExitReason::SwitchTo("_dotfiles".into()));
}

#[test]
fn goto_zoxide_multiple_collisions() {
    use tmux_sessions::adapters::tmux::RawSession;
    let fake = FakeTmuxSource {
        windows: vec![],
        sessions: vec![
            RawSession {
                session_name: "project".into(),
                attached: false,
                session_activity: None,
            },
            RawSession {
                session_name: "project-1".into(),
                attached: true,
                session_activity: None,
            },
        ],
        current_session_name: "project-1".into(),
        current_window_idx: "0".into(),
        existing_sessions: vec!["project".into(), "project-1".into()],
        fail_on: vec![],
    };
    let action = Action::goto_zoxide("/home/user/project".into(), "/home/user/project".into());
    let result = ActionExecutor::execute(&action, &fake).unwrap();
    assert_eq!(result, ExitReason::SwitchTo("project:99".into()));
}

#[test]
fn extract_session_name_characterization_matrix() {
    assert_eq!(extract_session_name("/"), "/");
    assert_eq!(extract_session_name("/home/user/proj/"), "proj");
    assert_eq!(extract_session_name("/a/b/nested"), "nested");
    assert_eq!(extract_session_name("simple-name"), "simple-name");
}

#[test]
fn sanitize_session_name_characterization_matrix() {
    assert_eq!(sanitize_session_name(".dotfiles"), "_dotfiles");
    assert_eq!(sanitize_session_name("my project@2026"), "my_project_2026");
    assert_eq!(sanitize_session_name("/"), "_");
    assert_eq!(sanitize_session_name("   "), "_");
}

#[test]
fn resolve_session_name_collision_characterization() {
    let existing = vec![
        "project".to_string(),
        "project-1".to_string(),
        "project-3".to_string(),
    ];

    assert_eq!(resolve_session_name("project", &existing), "project-2");
    assert_eq!(resolve_session_name("other", &existing), "other");
}
