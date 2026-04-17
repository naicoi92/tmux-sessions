use std::cell::RefCell;
use std::thread;
use std::time::Duration;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use tmux_sessions::adapters::tmux::{FakeTmuxSource, RawSession, RawWindow, TmuxSource};
use tmux_sessions::adapters::zoxide::{FakeZoxideSource, ZoxideSource};
use tmux_sessions::app::events::{map_key_to_action, HandledAction};
use tmux_sessions::app::executor::ActionExecutor;
use tmux_sessions::app::state::AppState;
use tmux_sessions::app::tmux_window_mapper::map_raw_windows_to_entries;
use tmux_sessions::domain::action::Action;
use tmux_sessions::domain::entry::{Entry, EntryType, SortPriority};
use tmux_sessions::domain::error::{ActionError, AdapterError};
use tmux_sessions::domain::snapshot::Snapshot;
use tmux_sessions::domain::sort::build_sorted_board;
use tmux_sessions::preview::generator::PreviewGenerator;
use tmux_sessions::preview::loader::AsyncPreviewLoader;
use tmux_sessions::preview::types::PreviewState;

#[derive(Debug, Clone, PartialEq, Eq)]
enum RecordedCall {
    ListSessions,
    CurrentSession,
    SelectWindow(String),
    NewSession { name: String, path: String },
    NewWindow { session: String, path: String },
    SwitchClient(String),
}

struct RecordingTmuxSource {
    sessions: Vec<RawSession>,
    calls: RefCell<Vec<RecordedCall>>,
    fail_on: Vec<RecordedCall>,
}

impl RecordingTmuxSource {
    fn new(sessions: Vec<RawSession>) -> Self {
        Self {
            sessions,
            calls: RefCell::new(Vec::new()),
            fail_on: vec![],
        }
    }

    fn new_with_failures(sessions: Vec<RawSession>, fail_on: Vec<RecordedCall>) -> Self {
        Self {
            sessions,
            calls: RefCell::new(Vec::new()),
            fail_on,
        }
    }

    fn calls(&self) -> Vec<RecordedCall> {
        self.calls.borrow().clone()
    }
}

impl TmuxSource for RecordingTmuxSource {
    fn list_windows(&self) -> Result<Vec<RawWindow>, AdapterError> {
        Ok(vec![])
    }

    fn list_sessions(&self) -> Result<Vec<RawSession>, AdapterError> {
        if self.fail_on.contains(&RecordedCall::ListSessions) {
            return Err(AdapterError::TmuxCommand {
                command: "list-sessions".into(),
                detail: "recorded fake list_sessions failure".into(),
            });
        }
        self.calls.borrow_mut().push(RecordedCall::ListSessions);
        Ok(self.sessions.clone())
    }

    fn current_session(&self) -> Result<String, AdapterError> {
        if self.fail_on.contains(&RecordedCall::CurrentSession) {
            return Err(AdapterError::TmuxCommand {
                command: "current-session".into(),
                detail: "recorded fake current-session failure".into(),
            });
        }
        Ok("s1".into())
    }

    fn current_window_index(&self) -> Result<String, AdapterError> {
        Ok("0".into())
    }

    fn has_session(&self, _name: &str) -> Result<bool, AdapterError> {
        Ok(self.sessions.iter().any(|s| s.session_name == _name))
    }

    fn select_window(&self, target: &str) -> Result<(), ActionError> {
        if self
            .fail_on
            .contains(&RecordedCall::SelectWindow(target.to_string()))
        {
            return Err(ActionError::GotoFailed {
                target: target.to_string(),
                detail: "recorded fake select-window failure".into(),
            });
        }
        self.calls
            .borrow_mut()
            .push(RecordedCall::SelectWindow(target.to_string()));
        Ok(())
    }

    fn new_session(&self, name: &str, path: &str) -> Result<(), ActionError> {
        if self.fail_on.contains(&RecordedCall::NewSession {
            name: name.to_string(),
            path: path.to_string(),
        }) {
            return Err(ActionError::GotoFailed {
                target: name.to_string(),
                detail: "recorded fake new-session failure".into(),
            });
        }
        self.calls.borrow_mut().push(RecordedCall::NewSession {
            name: name.to_string(),
            path: path.to_string(),
        });
        Ok(())
    }

    fn new_window(&self, session: &str, path: &str) -> Result<String, ActionError> {
        if self.fail_on.contains(&RecordedCall::NewWindow {
            session: session.to_string(),
            path: path.to_string(),
        }) {
            return Err(ActionError::GotoFailed {
                target: session.to_string(),
                detail: "recorded fake new-window failure".into(),
            });
        }
        self.calls.borrow_mut().push(RecordedCall::NewWindow {
            session: session.to_string(),
            path: path.to_string(),
        });
        Ok(format!("{session}:99"))
    }

    fn switch_client(&self, target: &str) -> Result<(), ActionError> {
        if self
            .fail_on
            .contains(&RecordedCall::SwitchClient(target.to_string()))
        {
            return Err(ActionError::GotoFailed {
                target: target.to_string(),
                detail: "recorded fake switch-client failure".into(),
            });
        }
        self.calls
            .borrow_mut()
            .push(RecordedCall::SwitchClient(target.to_string()));
        Ok(())
    }

    fn kill_window(&self, _target: &str) -> Result<(), ActionError> {
        Ok(())
    }

    fn kill_session(&self, _name: &str) -> Result<(), ActionError> {
        Ok(())
    }

    fn capture_pane(&self, _target: &str, _line_count: usize) -> Result<String, AdapterError> {
        Ok(String::new())
    }

    fn capture_pane_with_size(
        &self,
        _target: &str,
        _line_count: usize,
        _width: Option<u16>,
        _height: Option<u16>,
    ) -> Result<String, AdapterError> {
        Ok(String::new())
    }
}

#[test]
fn full_board_sort_with_fake_adapters() {
    let mut fake = FakeTmuxSource::new();
    fake.existing_sessions = vec!["s1".into()];
    fake.current_session_name = "s1".into();
    fake.current_window_idx = "0".into();
    fake.windows = vec![
        RawWindow {
            session_name: "s2".into(),
            window_index: "0".into(),
            window_name: "remote".into(),
            window_path: "/remote".into(),
            window_activity: None,
        },
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
            window_name: "edit".into(),
            window_path: "/home".into(),
            window_activity: None,
        },
    ];
    fake.sessions = vec![];

    let current_session = fake.current_session().unwrap();
    let current_idx = fake.current_window_index().unwrap();
    let tmux_entries =
        map_raw_windows_to_entries(fake.list_windows().unwrap(), &current_session, &current_idx);

    let zoxide = FakeZoxideSource::with_dirs(&["/home/project1", "/home/project2"]);
    let zoxide_entries = zoxide.directories(10).unwrap();

    let board = build_sorted_board(&current_session, &current_idx, tmux_entries, zoxide_entries);

    assert_eq!(board.len(), 5);
    assert_eq!(board[0].priority, SortPriority::CurrentWindow);
    assert_eq!(board[0].target, "s1:0");
    assert_eq!(board[1].priority, SortPriority::CurrentSessionOtherWindow);
    assert_eq!(board[1].target, "s1:1");
    assert_eq!(board[2].priority, SortPriority::OtherSessionWindow);
    assert_eq!(board[2].target, "s2:0");
    assert_eq!(board[3].priority, SortPriority::ZoxideDirectory);
    assert_eq!(board[4].priority, SortPriority::ZoxideDirectory);
}

#[test]
fn snapshot_from_board() {
    let entries = vec![
        Entry::zoxide("a".into(), "/a".into()),
        Entry::zoxide("b".into(), "/b".into()),
    ];
    let snap = Snapshot::new(entries, "s1".into(), "s1:0".into());
    assert_eq!(snap.len(), 2);
    assert_eq!(snap.current_session, "s1");
}

#[test]
fn action_goto_for_window_entry() {
    let entry = Entry::window(
        "s1".into(),
        "0".into(),
        "main".into(),
        "/path".into(),
        SortPriority::CurrentWindow,
        true,
        None,
    );
    let action = Action::goto_window(entry.target.clone(), entry.path.clone());
    assert_eq!(action.entry_type(), EntryType::Window);
}

#[test]
fn error_types_are_send_sync() {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<AdapterError>();
    assert_send_sync::<ActionError>();
}

#[test]
fn adapter_error_from_io() {
    let _err: AdapterError = std::io::Error::new(std::io::ErrorKind::NotFound, "not found").into();
}

#[test]
fn action_variants_exhaustive_match() {
    let actions = vec![
        Action::goto_window("t".into(), "/p".into()),
        Action::goto_zoxide("t".into(), "/p".into()),
        Action::kill_window("t".into()),
        Action::kill_zoxide("t".into()),
        Action::TogglePreview,
        Action::Reload,
        Action::Quit,
    ];

    for action in &actions {
        match action {
            Action::Goto { .. } => {}
            Action::Kill { .. } => {}
            Action::TogglePreview => {}
            Action::Reload => {}
            Action::Quit => {}
        }
    }
    assert_eq!(actions.len(), 7);
}

// Compile-time check: Action needs entry_type method for the integration test
trait ActionTypeCheck {
    fn entry_type(&self) -> EntryType;
}

impl ActionTypeCheck for Action {
    fn entry_type(&self) -> EntryType {
        match self {
            Action::Goto { entry_type, .. } => *entry_type,
            Action::Kill { entry_type, .. } => *entry_type,
            Action::TogglePreview | Action::Reload | Action::Quit => {
                panic!("utility actions have no entry type")
            }
        }
    }
}

#[test]
fn existing_item_enter_switches_correct_target() {
    let snapshot = Snapshot::new(
        vec![
            Entry::window(
                "s1".into(),
                "2".into(),
                "editor".into(),
                "/work/editor".into(),
                SortPriority::OtherSessionWindow,
                false,
                None,
            ),
            Entry::zoxide("project".into(), "/work/project".into()),
        ],
        "s1".into(),
        "s1:0".into(),
    );

    let state = AppState::new(snapshot);
    let enter = KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE);
    assert_eq!(map_key_to_action(enter), HandledAction::Goto);

    let action = state.build_action().expect("window action must exist");
    let tmux = RecordingTmuxSource::new(vec![]);
    let exit = ActionExecutor::execute(&action, &tmux).expect("goto should execute");

    assert_eq!(
        exit,
        tmux_sessions::app::executor::ExitReason::SwitchTo("s1:2".into())
    );
    assert_eq!(
        tmux.calls(),
        vec![
            RecordedCall::SelectWindow("s1:2".into()),
            RecordedCall::SwitchClient("s1".into()),
        ]
    );
}

#[test]
fn zoxide_enter_creates_single_session_then_switches() {
    let snapshot = Snapshot::new(
        vec![
            Entry::window(
                "s1".into(),
                "2".into(),
                "editor".into(),
                "/work/editor".into(),
                SortPriority::OtherSessionWindow,
                false,
                None,
            ),
            Entry::zoxide("project".into(), "/work/project".into()),
        ],
        "s1".into(),
        "s1:0".into(),
    );

    let mut state = AppState::new(snapshot);
    state.selected_index = 1;
    let enter = KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE);
    assert_eq!(map_key_to_action(enter), HandledAction::Goto);

    let action = state.build_action().expect("zoxide action must exist");
    let tmux = RecordingTmuxSource::new(vec![]);
    let exit = ActionExecutor::execute(&action, &tmux).expect("goto should execute");

    assert_eq!(
        exit,
        tmux_sessions::app::executor::ExitReason::SwitchTo("project".into())
    );
    assert_eq!(
        tmux.calls(),
        vec![
            RecordedCall::NewSession {
                name: "project".into(),
                path: "/work/project".into(),
            },
            RecordedCall::SwitchClient("project".into()),
        ]
    );
}

#[test]
fn grouped_existing_switch_flow_resolves_actionable_child_target() {
    let snapshot = Snapshot::new(
        vec![
            Entry::window(
                "team".into(),
                "0".into(),
                "editor".into(),
                "/work/team".into(),
                SortPriority::OtherSessionWindow,
                false,
                None,
            ),
            Entry::window(
                "team".into(),
                "1".into(),
                "logs".into(),
                "/work/team".into(),
                SortPriority::OtherSessionWindow,
                false,
                None,
            ),
            Entry::zoxide("project".into(), "/work/project".into()),
        ],
        "s1".into(),
        "s1:0".into(),
    );

    let mut state = AppState::new(snapshot);
    state.selected_index = 1;
    let action = state
        .build_enter_action()
        .expect("second actionable row should exist");

    let tmux = RecordingTmuxSource::new(vec![]);
    let exit = ActionExecutor::execute(&action, &tmux).expect("grouped window goto should execute");

    assert_eq!(
        exit,
        tmux_sessions::app::executor::ExitReason::SwitchTo("team:1".into())
    );
    assert_eq!(
        tmux.calls(),
        vec![
            RecordedCall::SelectWindow("team:1".into()),
            RecordedCall::SwitchClient("team".into()),
        ]
    );
}

#[test]
fn zoxide_create_flow_uses_gap_filling_name_when_sessions_exist() {
    let snapshot = Snapshot::new(
        vec![Entry::zoxide("project".into(), "/work/project".into())],
        "s1".into(),
        "s1:0".into(),
    );
    let state = AppState::new(snapshot);
    let action = state
        .build_enter_action()
        .expect("zoxide action must exist");

    let tmux = RecordingTmuxSource::new(vec![
        RawSession {
            session_name: "project".into(),
            attached: true,
        },
        RawSession {
            session_name: "project-1".into(),
            attached: false,
        },
    ]);

    let exit = ActionExecutor::execute(&action, &tmux).expect("zoxide goto should execute");
    assert_eq!(
        exit,
        tmux_sessions::app::executor::ExitReason::SwitchTo("project:99".into())
    );
    assert_eq!(
        tmux.calls(),
        vec![
            RecordedCall::NewWindow {
                session: "project".into(),
                path: "/work/project".into(),
            },
            RecordedCall::SelectWindow("project:99".into()),
            RecordedCall::SwitchClient("project".into()),
        ]
    );
}

#[test]
fn grouped_rows_keep_header_non_actionable_and_selection_actionable() {
    let snapshot = Snapshot::new(
        vec![
            Entry::window(
                "grouped".into(),
                "0".into(),
                "editor".into(),
                "/grouped".into(),
                SortPriority::OtherSessionWindow,
                false,
                None,
            ),
            Entry::window(
                "grouped".into(),
                "1".into(),
                "shell".into(),
                "/grouped".into(),
                SortPriority::OtherSessionWindow,
                false,
                None,
            ),
            Entry::zoxide("z".into(), "/z".into()),
        ],
        "s1".into(),
        "s1:0".into(),
    );

    let state = AppState::new(snapshot);
    let rows = state.filtered_rows();

    assert!(!rows[0].is_actionable());
    assert!(rows[1].is_actionable());
    assert!(rows[2].is_actionable());
    assert!(rows[3].is_actionable());
    assert_eq!(state.selected_visible_index(), Some(1));
    assert_eq!(state.selected_entry().unwrap().target, "grouped:0");
}

#[test]
fn rapid_preview_requests_keep_latest_target_content() {
    let generator = PreviewGenerator::with_factory(Box::new(FakeTmuxSource::new()), || {
        Box::new(FakeTmuxSource::new())
    });
    let mut loader = AsyncPreviewLoader::new(generator);

    let first = Entry::window(
        "first".into(),
        "0".into(),
        "main".into(),
        "/tmp".into(),
        SortPriority::OtherSessionWindow,
        false,
        None,
    );
    let second = Entry::window(
        "second".into(),
        "0".into(),
        "main".into(),
        "/tmp".into(),
        SortPriority::OtherSessionWindow,
        false,
        None,
    );

    loader.request(&first, None);
    loader.request(&second, None);

    let mut result = None;
    for _ in 0..40 {
        if let Some(content) = loader.poll() {
            result = Some(content);
            break;
        }
        thread::sleep(Duration::from_millis(25));
    }

    match result.expect("latest preview should eventually resolve") {
        PreviewState::DirectoryListing(content) => {
            assert_eq!(content.source, "tmux-fallback");
            assert_eq!(content.path, "/tmp");
        }
        other => panic!("expected DirectoryListing for latest target, got: {other:?}"),
    }
}

#[test]
fn existing_switch_flow_surfaces_tmux_select_error() {
    let snapshot = Snapshot::new(
        vec![Entry::window(
            "s1".into(),
            "2".into(),
            "editor".into(),
            "/work/editor".into(),
            SortPriority::OtherSessionWindow,
            false,
            None,
        )],
        "s1".into(),
        "s1:0".into(),
    );

    let state = AppState::new(snapshot);
    let action = state
        .build_enter_action()
        .expect("window action must exist");
    let tmux = RecordingTmuxSource::new_with_failures(
        vec![],
        vec![RecordedCall::SelectWindow("s1:2".into())],
    );

    let err = ActionExecutor::execute(&action, &tmux).expect_err("select failure should surface");
    match err {
        ActionError::GotoFailed { target, detail } => {
            assert_eq!(target, "s1:2");
            assert!(detail.contains("select-window"));
        }
        other => panic!("expected GotoFailed, got: {other:?}"),
    }
}

#[test]
fn zoxide_create_flow_surfaces_session_listing_error() {
    let snapshot = Snapshot::new(
        vec![Entry::zoxide("project".into(), "/work/project".into())],
        "s1".into(),
        "s1:0".into(),
    );
    let state = AppState::new(snapshot);
    let action = state
        .build_enter_action()
        .expect("zoxide action must exist");

    let tmux = RecordingTmuxSource::new_with_failures(
        vec![],
        vec![RecordedCall::NewSession {
            name: "project".into(),
            path: "/work/project".into(),
        }],
    );
    let err =
        ActionExecutor::execute(&action, &tmux).expect_err("new session failure should surface");

    match err {
        ActionError::GotoFailed { target, detail } => {
            assert_eq!(target, "project");
            assert!(detail.contains("new-session"));
        }
        other => panic!("expected GotoFailed, got: {other:?}"),
    }
}

#[test]
fn zoxide_create_flow_surfaces_new_session_error() {
    let snapshot = Snapshot::new(
        vec![Entry::zoxide("project".into(), "/work/project".into())],
        "s1".into(),
        "s1:0".into(),
    );
    let state = AppState::new(snapshot);
    let action = state
        .build_enter_action()
        .expect("zoxide action must exist");

    let tmux = RecordingTmuxSource::new_with_failures(
        vec![],
        vec![RecordedCall::NewSession {
            name: "project".into(),
            path: "/work/project".into(),
        }],
    );
    let err =
        ActionExecutor::execute(&action, &tmux).expect_err("new session failure should surface");

    match err {
        ActionError::GotoFailed { target, detail } => {
            assert_eq!(target, "project");
            assert!(detail.contains("new-session"));
        }
        other => panic!("expected GotoFailed, got: {other:?}"),
    }
}

#[test]
fn zoxide_create_flow_surfaces_new_window_error() {
    let snapshot = Snapshot::new(
        vec![Entry::zoxide("project".into(), "/work/project".into())],
        "s1".into(),
        "s1:0".into(),
    );
    let state = AppState::new(snapshot);
    let action = state
        .build_enter_action()
        .expect("zoxide action must exist");

    let tmux = RecordingTmuxSource::new_with_failures(
        vec![RawSession {
            session_name: "project".into(),
            attached: false,
        }],
        vec![RecordedCall::NewWindow {
            session: "project".into(),
            path: "/work/project".into(),
        }],
    );
    let err =
        ActionExecutor::execute(&action, &tmux).expect_err("new window failure should surface");

    match err {
        ActionError::GotoFailed { target, detail } => {
            assert_eq!(target, "project");
            assert!(detail.contains("new-window"));
        }
        other => panic!("expected GotoFailed, got: {other:?}"),
    }
}

#[test]
fn zoxide_create_flow_surfaces_switch_client_error() {
    let snapshot = Snapshot::new(
        vec![Entry::zoxide("project".into(), "/work/project".into())],
        "s1".into(),
        "s1:0".into(),
    );
    let state = AppState::new(snapshot);
    let action = state
        .build_enter_action()
        .expect("zoxide action must exist");

    let tmux = RecordingTmuxSource::new_with_failures(
        vec![],
        vec![RecordedCall::SwitchClient("project".into())],
    );
    let err =
        ActionExecutor::execute(&action, &tmux).expect_err("switch client failure should surface");

    match err {
        ActionError::GotoFailed { target, detail } => {
            assert_eq!(target, "project");
            assert!(detail.contains("switch-client"));
        }
        other => panic!("expected GotoFailed, got: {other:?}"),
    }
}
