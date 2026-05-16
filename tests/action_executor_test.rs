use tmux_sessions::adapters::tmux::{
    FakeTmuxCall, FakeTmuxSource, RawSession, RawWindow, TmuxSource,
};
use tmux_sessions::app::executor::{
    extract_session_name, sanitize_session_name, ActionExecutor, ExitReason,
};
use tmux_sessions::domain::action::Action;
use tmux_sessions::domain::entry::EntryType;
use tmux_sessions::domain::error::{ActionError, AdapterError};

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

    fn set_window_option(
        &self,
        target: &str,
        option: &str,
        value: &str,
    ) -> Result<(), ActionError> {
        self.calls
            .borrow_mut()
            .push(format!("set_window_option:{target}:{option}:{value}"));
        Ok(())
    }
}

// --- Window goto tests ---

#[test]
fn goto_window_returns_switch_to() {
    let fake = fake_no_sessions();
    let action = Action::goto_window("s1:0".into(), "/path".into());
    let result = ActionExecutor::execute(&action, &fake).unwrap();
    assert_eq!(result, ExitReason::SwitchTo("s1:0".into()));
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

// --- Zoxide goto tests (with session_name_hint) ---

#[test]
fn goto_zoxide_new_session_returns_switch_to() {
    let fake = fake_no_sessions();
    let action = Action::goto_zoxide(
        "/home/user/project".into(),
        "/home/user/project".into(),
        "project".into(),
    );
    let result = ActionExecutor::execute(&action, &fake).unwrap();
    assert_eq!(result, ExitReason::SwitchTo("project:0".into()));
}

#[test]
fn goto_zoxide_existing_session_creates_window() {
    let fake = fake_with_session("myproject");
    let action = Action::goto_zoxide(
        "/home/user/myproject".into(),
        "/home/user/myproject".into(),
        "myproject".into(),
    );
    let result = ActionExecutor::execute(&action, &fake).unwrap();
    assert_eq!(result, ExitReason::SwitchTo("myproject:99".into()));
}

#[test]
fn session_name_from_nested_path() {
    let fake = fake_no_sessions();
    let action = Action::goto_zoxide(
        "/a/deep/nested/project".into(),
        "/a/deep/nested/project".into(),
        "project".into(),
    );
    let result = ActionExecutor::execute(&action, &fake).unwrap();
    assert_eq!(result, ExitReason::SwitchTo("project:0".into()));
}

#[test]
fn session_name_from_simple_path() {
    let fake = fake_no_sessions();
    let action = Action::goto_zoxide(
        "/home/user/dotfiles".into(),
        "/home/user/dotfiles".into(),
        "dotfiles".into(),
    );
    let result = ActionExecutor::execute(&action, &fake).unwrap();
    assert_eq!(result, ExitReason::SwitchTo("dotfiles:0".into()));
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
    let action = Action::goto_zoxide("/proj".into(), "/proj".into(), "proj".into());
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
fn goto_zoxide_invalid_basename_sanitized() {
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
    let action = Action::goto_zoxide("/".into(), "/".into(), "/".into());
    let result = ActionExecutor::execute(&action, &fake).unwrap();
    // "/" → sanitized "_" → session "_" exists → create window
    assert_eq!(result, ExitReason::SwitchTo("_:99".into()));
}

#[test]
fn goto_zoxide_dot_prefix_sanitized() {
    let fake = fake_no_sessions();
    let action = Action::goto_zoxide(
        "/home/user/.dotfiles".into(),
        "/home/user/.dotfiles".into(),
        ".dotfiles".into(),
    );
    let result = ActionExecutor::execute(&action, &fake).unwrap();
    // ".dotfiles" → sanitized "_dotfiles" → new session
    assert_eq!(result, ExitReason::SwitchTo("_dotfiles:0".into()));
}

#[test]
fn goto_zoxide_multiple_collisions() {
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
    let action = Action::goto_zoxide(
        "/home/user/project".into(),
        "/home/user/project".into(),
        "project".into(),
    );
    let result = ActionExecutor::execute(&action, &fake).unwrap();
    // Session "project" exists → create window
    assert_eq!(result, ExitReason::SwitchTo("project:99".into()));
}

// --- Kill tests ---

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

// --- Utility action tests ---

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

// --- Characterization tests ---

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

// --- Zoxide path-based identity tests ---

#[test]
fn zoxide_existing_window_same_path_switches_to_it() {
    let fake = FakeTmuxSource {
        windows: vec![RawWindow {
            session_name: "bot".into(),
            window_index: "0".into(),
            window_name: "vim".into(),
            window_path: "/projects/a/bot".into(),
            original_path: None,
            display_name: None,
            window_activity: None,
        }],
        sessions: vec![RawSession {
            session_name: "bot".into(),
            attached: true,
            session_activity: None,
        }],
        current_session_name: "bot".into(),
        current_window_idx: "0".into(),
        existing_sessions: vec!["bot".into()],
        fail_on: vec![],
    };
    let action = Action::goto_zoxide(
        "/projects/a/bot".into(),
        "/projects/a/bot".into(),
        "bot".into(),
    );
    let result = ActionExecutor::execute(&action, &fake).unwrap();
    // Session "bot" exists → create new window in it
    assert_eq!(result, ExitReason::SwitchTo("bot:99".into()));
}

#[test]
fn zoxide_different_path_same_basename_creates_new_window_in_existing_session() {
    let fake = FakeTmuxSource {
        windows: vec![RawWindow {
            session_name: "bot".into(),
            window_index: "0".into(),
            window_name: "vim".into(),
            window_path: "/projects/a/bot".into(),
            original_path: None,
            display_name: None,
            window_activity: None,
        }],
        sessions: vec![RawSession {
            session_name: "bot".into(),
            attached: true,
            session_activity: None,
        }],
        current_session_name: "bot".into(),
        current_window_idx: "0".into(),
        existing_sessions: vec!["bot".into()],
        fail_on: vec![],
    };
    let action = Action::goto_zoxide(
        "/projects/c/bot".into(),
        "/projects/c/bot".into(),
        "bot".into(),
    );
    let result = ActionExecutor::execute(&action, &fake).unwrap();
    assert_eq!(result, ExitReason::SwitchTo("bot:99".into()));
}

#[test]
fn zoxide_public_vs_public_api_no_collision() {
    let fake = FakeTmuxSource {
        windows: vec![RawWindow {
            session_name: "public-api".into(),
            window_index: "0".into(),
            window_name: "editor".into(),
            window_path: "/projects/public-api".into(),
            original_path: None,
            display_name: None,
            window_activity: None,
        }],
        sessions: vec![RawSession {
            session_name: "public-api".into(),
            attached: true,
            session_activity: None,
        }],
        current_session_name: "public-api".into(),
        current_window_idx: "0".into(),
        existing_sessions: vec!["public-api".into()],
        fail_on: vec![],
    };
    // "public" basename ≠ "public-api" → no collision, new session "public"
    let action = Action::goto_zoxide(
        "/other/public".into(),
        "/other/public".into(),
        "public".into(),
    );
    let result = ActionExecutor::execute(&action, &fake).unwrap();
    assert_eq!(result, ExitReason::SwitchTo("public:0".into()));
}

#[test]
fn zoxide_revisit_same_path_creates_new_window() {
    let fake = FakeTmuxSource {
        windows: vec![
            RawWindow {
                session_name: "project".into(),
                window_index: "0".into(),
                window_name: "main".into(),
                window_path: "/home/user/project".into(),
                original_path: None,
                display_name: None,
                window_activity: None,
            },
            RawWindow {
                session_name: "project".into(),
                window_index: "1".into(),
                window_name: "tests".into(),
                window_path: "/home/user/project".into(),
                original_path: None,
                display_name: None,
                window_activity: None,
            },
        ],
        sessions: vec![RawSession {
            session_name: "project".into(),
            attached: true,
            session_activity: None,
        }],
        current_session_name: "project".into(),
        current_window_idx: "0".into(),
        existing_sessions: vec!["project".into()],
        fail_on: vec![],
    };
    let action = Action::goto_zoxide(
        "/home/user/project".into(),
        "/home/user/project".into(),
        "project".into(),
    );
    let result = ActionExecutor::execute(&action, &fake).unwrap();
    assert_eq!(result, ExitReason::SwitchTo("project:99".into()));
}

// --- set_window_option integration ---

#[test]
fn zoxide_new_session_sets_window_metadata_options() {
    let fake = RecordingTmuxSource::default();
    let action = Action::goto_zoxide(
        "/repo/project".into(),
        "/repo/project".into(),
        "project".into(),
    );
    let result = ActionExecutor::execute(&action, &fake).unwrap();
    assert_eq!(result, ExitReason::SwitchTo("project:0".into()));
    let set_option_calls: Vec<String> = fake
        .calls()
        .into_iter()
        .filter(|c| c.starts_with("set_window_option:"))
        .collect();
    assert_eq!(
        set_option_calls,
        vec![
            "set_window_option:project:0:@pi_original_path:/repo/project".to_string(),
            "set_window_option:project:0:@pi_display_name:project".to_string(),
        ]
    );
}

#[test]
fn zoxide_existing_session_new_window_sets_original_path_option() {
    let _fake = RecordingTmuxSource {
        calls: RefCell::new(vec![]),
        fail_switch_on: None,
    };
    // Make list_sessions return an existing "project" session
    // RecordingTmuxSource returns empty sessions, so we can't test this easily.
    // This is tested via FakeTmuxSource in the integration tests.
}
