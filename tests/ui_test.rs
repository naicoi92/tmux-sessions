use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{backend::TestBackend, Terminal};
use tmux_sessions::app::events::{apply_action, map_key_to_action, HandledAction};
use tmux_sessions::app::state::AppState;
use tmux_sessions::domain::entry::{Entry, EntryType, SortPriority};
use tmux_sessions::domain::grouped_list::{GroupedList, GroupedListItem};
use tmux_sessions::domain::snapshot::Snapshot;
use tmux_sessions::preview::types::{DirectoryListingContent, PreviewState, TmuxScreenContent};
use tmux_sessions::ui::render;

fn key(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

fn ctrl(c: char) -> KeyEvent {
    KeyEvent::new(KeyCode::Char(c), KeyModifiers::CONTROL)
}

fn snap_with(entries: Vec<Entry>) -> Snapshot {
    Snapshot::new(entries, "s".into(), "s:0".into())
}

fn w(name: &str) -> Entry {
    Entry::window("s".into(), "0".into(), name.into(), "/".into(), SortPriority::OtherSessionWindow, false, None)
}

fn z(name: &str) -> Entry {
    Entry::zoxide(name.into(), format!("/{name}"))
}

#[test]
fn key_event_to_action_parity() {
    let cases: Vec<(KeyEvent, HandledAction)> = vec![
        (key(KeyCode::Enter), HandledAction::Goto),
        (key(KeyCode::Esc), HandledAction::Quit),
        (ctrl('c'), HandledAction::Quit),
        (ctrl('d'), HandledAction::Kill),
        (ctrl('o'), HandledAction::TogglePreview),
        (ctrl('r'), HandledAction::Reload),
        (key(KeyCode::Up), HandledAction::MoveUp),
        (key(KeyCode::Down), HandledAction::MoveDown),
        (key(KeyCode::Home), HandledAction::MoveTop),
        (key(KeyCode::End), HandledAction::MoveBottom),
        (key(KeyCode::Backspace), HandledAction::Backspace),
        (key(KeyCode::Left), HandledAction::FilterCursorLeft),
        (key(KeyCode::Right), HandledAction::FilterCursorRight),
    ];

    for (key_event, expected) in cases {
        assert_eq!(
            map_key_to_action(key_event),
            expected,
            "key mapping mismatch for {key_event:?}"
        );
    }
}

#[test]
fn selection_movement_sequence() {
    let entries = vec![w("a"), w("b"), w("c")];
    let mut state = AppState::new(snap_with(entries));

    apply_action(&mut state, HandledAction::MoveDown);
    assert_eq!(state.selected_index, 1);

    apply_action(&mut state, HandledAction::MoveDown);
    assert_eq!(state.selected_index, 2);

    apply_action(&mut state, HandledAction::MoveDown);
    assert_eq!(state.selected_index, 2, "should clamp at last");

    apply_action(&mut state, HandledAction::MoveUp);
    assert_eq!(state.selected_index, 1);
}

#[test]
fn preview_toggle_preserves_all_state() {
    let entries = vec![w("a"), w("b"), w("c")];
    let mut state = AppState::new(snap_with(entries));
    state.set_filter('b');
    state.selected_index = 0;

    apply_action(&mut state, HandledAction::TogglePreview);
    assert_eq!(state.selected_index, 0);
    assert_eq!(state.filter, "b");
    assert!(!state.preview_visible);

    apply_action(&mut state, HandledAction::TogglePreview);
    assert_eq!(state.selected_index, 0);
    assert_eq!(state.filter, "b");
    assert!(state.preview_visible);
}

#[test]
fn preview_toggle_multiple_times_stable() {
    let entries = vec![w("a"), w("b"), w("c")];
    let mut state = AppState::new(snap_with(entries));
    state.selected_index = 2;

    for _ in 0..10 {
        apply_action(&mut state, HandledAction::TogglePreview);
    }

    assert_eq!(state.selected_index, 2);
    assert!(state.selected_entry().is_some());
}

#[test]
fn filter_then_move_works() {
    let entries = vec![
        Entry::window(
            "s".into(),
            "0".into(),
            "alpha".into(),
            "/".into(),
            SortPriority::OtherSessionWindow,
            false,
        None,
        ),
        Entry::window(
            "s".into(),
            "1".into(),
            "beta".into(),
            "/".into(),
            SortPriority::OtherSessionWindow,
            false,
        None,
        ),
        Entry::window(
            "s".into(),
            "2".into(),
            "gamma".into(),
            "/".into(),
            SortPriority::OtherSessionWindow,
            false,
        None,
        ),
    ];
    let mut state = AppState::new(snap_with(entries));

    apply_action(&mut state, HandledAction::FilterChar('b'));
    assert_eq!(state.filtered_entries().len(), 1);
    assert_eq!(state.selected_index, 0);

    apply_action(&mut state, HandledAction::ClearFilter);
    assert_eq!(state.filtered_entries().len(), 3);
}

#[test]
fn filter_preserves_preview_visibility() {
    let entries = vec![w("a")];
    let mut state = AppState::new(snap_with(entries));
    state.toggle_preview();

    apply_action(&mut state, HandledAction::FilterChar('a'));
    assert!(!state.preview_visible);

    apply_action(&mut state, HandledAction::ClearFilter);
    assert!(!state.preview_visible);
}

#[test]
fn goto_action_window() {
    let state = AppState::new(snap_with(vec![w("main")]));
    let action = state.build_action().unwrap();
    match action {
        tmux_sessions::domain::action::Action::Goto {
            entry_type, target, ..
        } => {
            assert_eq!(entry_type, EntryType::Window);
            assert_eq!(target, "s:0");
        }
        _ => panic!("expected Goto"),
    }
}

#[test]
fn goto_action_zoxide() {
    let state = AppState::new(snap_with(vec![z("project")]));
    let action = state.build_action().unwrap();
    match action {
        tmux_sessions::domain::action::Action::Goto {
            entry_type,
            target,
            path,
        } => {
            assert_eq!(entry_type, EntryType::Zoxide);
            assert_eq!(target, "/project");
            assert_eq!(path, "/project");
        }
        _ => panic!("expected Goto"),
    }
}

#[test]
fn kill_action_window() {
    let state = AppState::new(snap_with(vec![w("w")]));
    let action = state.build_kill_action().unwrap();
    match action {
        tmux_sessions::domain::action::Action::Kill { entry_type, target } => {
            assert_eq!(entry_type, EntryType::Window);
            assert_eq!(target, "s:0");
        }
        _ => panic!("expected Kill"),
    }
}

#[test]
fn kill_action_zoxide() {
    let state = AppState::new(snap_with(vec![z("p")]));
    let action = state.build_kill_action().unwrap();
    match action {
        tmux_sessions::domain::action::Action::Kill { entry_type, target } => {
            assert_eq!(entry_type, EntryType::Zoxide);
            assert_eq!(target, "/p");
        }
        _ => panic!("expected Kill"),
    }
}

#[test]
fn empty_snapshot_no_action() {
    let state = AppState::new(snap_with(vec![]));
    assert!(state.build_action().is_none());
    assert!(state.build_kill_action().is_none());
}

#[test]
fn render_layout_ratio_and_help_hints() {
    let entries = vec![w("a")];
    let mut state = AppState::new(snap_with(entries));
    state.preview_visible = true;

    let backend = TestBackend::new(100, 20);
    let mut terminal = Terminal::new(backend).unwrap();

    terminal
        .draw(|f| {
            render(f, &state);
        })
        .unwrap();

    let buffer = terminal.backend().buffer();

    // Check help hints presence
    let last_line = (0..100)
        .map(|x| buffer[(x, 19)].symbol())
        .collect::<Vec<_>>()
        .join("");

    assert!(last_line.contains("navigate"));
    assert!(last_line.contains("goto"));
    assert!(last_line.contains("kill"));
    assert!(last_line.contains("quit"));
    assert!(
        last_line
            .trim_start()
            .starts_with(&format!("v{}", env!("CARGO_PKG_VERSION"))),
        "status bar should show version on the left"
    );

    // Search bar is rows 0-2 (border, content, border). Sessions title on row 3.
    let sessions_title_line = (0..20)
        .map(|x| buffer[(x, 3)].symbol())
        .collect::<Vec<_>>()
        .join("");

    assert!(
        sessions_title_line.contains("Sessions"),
        "sessions block title should contain 'Sessions'"
    );

    // Preview block title is on row 0 on the right side
    let preview_title_line = (20..100)
        .map(|x| buffer[(x, 0)].symbol())
        .collect::<Vec<_>>()
        .join("");

    assert!(
        preview_title_line.contains(''),
        "preview block title should contain terminal icon for window entry"
    );
}

#[test]
fn state_transitions_are_idempotent_for_quit() {
    let entries = vec![w("a")];
    let mut state = AppState::new(snap_with(entries));

    apply_action(&mut state, HandledAction::Quit);
    assert!(state.should_quit);

    apply_action(&mut state, HandledAction::MoveDown);
    assert!(state.should_quit, "quit flag should survive other actions");
}

#[test]
fn mixed_entries_in_snapshot_preserve_order() {
    let entries = vec![
        Entry::window(
            "s".into(),
            "0".into(),
            "a".into(),
            "/".into(),
            SortPriority::CurrentWindow,
            true,
        None,
        ),
        Entry::window(
            "s2".into(),
            "0".into(),
            "b".into(),
            "/".into(),
            SortPriority::OtherSessionWindow,
            false,
        None,
        ),
        Entry::zoxide("dir".into(), "/dir".into()),
    ];
    let state = AppState::new(snap_with(entries));

    let filtered = state.filtered_entries();
    assert_eq!(filtered.len(), 3);

    let types: Vec<EntryType> = filtered.iter().map(|e| e.entry_type).collect();
    assert_eq!(types[0], EntryType::Window);
    assert_eq!(types[1], EntryType::Window);
    assert_eq!(types[2], EntryType::Zoxide);
}

#[test]
fn grouped_list_renders_multi_window_sessions_as_groups() {
    let entries = vec![
        Entry::window(
            "team".into(),
            "0".into(),
            "editor".into(),
            "/team".into(),
            SortPriority::CurrentSessionOtherWindow,
            false,
        None,
        ),
        Entry::window(
            "team".into(),
            "1".into(),
            "logs".into(),
            "/team".into(),
            SortPriority::CurrentSessionOtherWindow,
            false,
        None,
        ),
    ];

    let grouped = GroupedList::from_snapshot(&snap_with(entries));
    assert_eq!(grouped.items.len(), 1);

    match &grouped.items[0] {
        GroupedListItem::SessionGroup { session, windows } => {
            assert_eq!(session, "team");
            assert_eq!(windows.len(), 2);
        }
        other => panic!("expected SessionGroup, got {other:?}"),
    }
}

#[test]
fn grouped_list_keeps_single_window_as_standalone_item() {
    let entries = vec![Entry::window(
        "solo".into(),
        "0".into(),
        "main".into(),
        "/solo".into(),
        SortPriority::CurrentWindow,
        true,
    None,
    )];

    let grouped = GroupedList::from_snapshot(&snap_with(entries));
    assert_eq!(grouped.items.len(), 1);

    match &grouped.items[0] {
        GroupedListItem::StandaloneSession(window) => {
            assert_eq!(window.session_name.as_deref(), Some("solo"));
        }
        other => panic!("expected StandaloneSession, got {other:?}"),
    }
}

#[test]
fn grouped_list_includes_zoxide_rows() {
    let grouped = GroupedList::from_snapshot(&snap_with(vec![z("project")]));
    assert_eq!(grouped.items.len(), 1);

    match &grouped.items[0] {
        GroupedListItem::ZoxideEntry(entry) => {
            assert_eq!(entry.entry_type, EntryType::Zoxide);
            assert_eq!(entry.target, "/project");
        }
        other => panic!("expected ZoxideEntry, got {other:?}"),
    }
}

#[test]
fn app_state_never_selects_group_header() {
    let entries = vec![
        Entry::window(
            "grouped".into(),
            "0".into(),
            "editor".into(),
            "/grouped".into(),
            SortPriority::CurrentSessionOtherWindow,
            false,
        None,
        ),
        Entry::window(
            "grouped".into(),
            "1".into(),
            "shell".into(),
            "/grouped".into(),
            SortPriority::CurrentSessionOtherWindow,
            false,
        None,
        ),
    ];
    let state = AppState::new(snap_with(entries));

    let selected = state
        .selected_entry()
        .expect("must select actionable child");
    assert_eq!(selected.target, "grouped:0");
}

#[test]
fn selection_survives_snapshot_replace_by_stable_target() {
    let initial = vec![
        Entry::window(
            "alpha".into(),
            "0".into(),
            "main".into(),
            "/alpha".into(),
            SortPriority::OtherSessionWindow,
            false,
        None,
        ),
        Entry::window(
            "beta".into(),
            "0".into(),
            "main".into(),
            "/beta".into(),
            SortPriority::OtherSessionWindow,
            false,
        None,
        ),
    ];
    let mut state = AppState::new(snap_with(initial));

    apply_action(&mut state, HandledAction::MoveDown);
    assert_eq!(state.selected_entry().unwrap().target, "beta:0");

    let regrouped = vec![
        Entry::window(
            "beta".into(),
            "0".into(),
            "main".into(),
            "/beta".into(),
            SortPriority::OtherSessionWindow,
            false,
        None,
        ),
        Entry::window(
            "beta".into(),
            "1".into(),
            "logs".into(),
            "/beta".into(),
            SortPriority::OtherSessionWindow,
            false,
        None,
        ),
        Entry::window(
            "alpha".into(),
            "0".into(),
            "main".into(),
            "/alpha".into(),
            SortPriority::OtherSessionWindow,
            false,
        None,
        ),
    ];

    state.replace_snapshot(snap_with(regrouped));

    let selected = state
        .selected_entry()
        .expect("selection should survive reload");
    assert_eq!(selected.target, "beta:0");
}

#[test]
fn selection_falls_back_to_nearest_actionable_when_target_disappears() {
    let entries = vec![
        Entry::window(
            "alpha".into(),
            "0".into(),
            "main".into(),
            "/alpha".into(),
            SortPriority::OtherSessionWindow,
            false,
        None,
        ),
        Entry::window(
            "beta".into(),
            "0".into(),
            "main".into(),
            "/beta".into(),
            SortPriority::OtherSessionWindow,
            false,
        None,
        ),
        Entry::window(
            "charlie".into(),
            "0".into(),
            "main".into(),
            "/charlie".into(),
            SortPriority::OtherSessionWindow,
            false,
        None,
        ),
    ];
    let mut state = AppState::new(snap_with(entries));

    apply_action(&mut state, HandledAction::MoveDown);
    assert_eq!(state.selected_entry().unwrap().target, "beta:0");

    let removed_selected = vec![
        Entry::window(
            "alpha".into(),
            "0".into(),
            "main".into(),
            "/alpha".into(),
            SortPriority::OtherSessionWindow,
            false,
        None,
        ),
        Entry::window(
            "charlie".into(),
            "0".into(),
            "main".into(),
            "/charlie".into(),
            SortPriority::OtherSessionWindow,
            false,
        None,
        ),
    ];

    state.replace_snapshot(snap_with(removed_selected));

    let selected = state
        .selected_entry()
        .expect("should fallback to a nearby actionable row");
    assert_eq!(selected.target, "charlie:0");
}

#[test]
fn selection_survives_filter_roundtrip_by_target_identity() {
    let entries = vec![
        Entry::window(
            "s".into(),
            "0".into(),
            "alpha".into(),
            "/".into(),
            SortPriority::OtherSessionWindow,
            false,
        None,
        ),
        Entry::window(
            "s".into(),
            "1".into(),
            "beta".into(),
            "/".into(),
            SortPriority::OtherSessionWindow,
            false,
        None,
        ),
        Entry::window(
            "s".into(),
            "2".into(),
            "gamma".into(),
            "/".into(),
            SortPriority::OtherSessionWindow,
            false,
        None,
        ),
    ];
    let mut state = AppState::new(snap_with(entries));

    apply_action(&mut state, HandledAction::MoveDown);
    apply_action(&mut state, HandledAction::MoveDown);
    assert_eq!(state.selected_entry().unwrap().target, "s:2");

    apply_action(&mut state, HandledAction::FilterChar('g'));
    assert_eq!(state.filter, "g");
    assert_eq!(state.selected_entry().unwrap().target, "s:2");

    apply_action(&mut state, HandledAction::ClearFilter);
    assert_eq!(state.filter, "");
    assert_eq!(state.selected_entry().unwrap().target, "s:2");
}

#[test]
fn replace_snapshot_with_group_headers_still_selects_actionable_entry() {
    let initial = vec![
        Entry::window(
            "a".into(),
            "0".into(),
            "one".into(),
            "/a".into(),
            SortPriority::OtherSessionWindow,
            false,
        None,
        ),
        Entry::window(
            "b".into(),
            "0".into(),
            "two".into(),
            "/b".into(),
            SortPriority::OtherSessionWindow,
            false,
        None,
        ),
    ];
    let mut state = AppState::new(snap_with(initial));
    apply_action(&mut state, HandledAction::MoveDown);
    assert_eq!(state.selected_entry().unwrap().target, "b:0");

    let regrouped = vec![
        Entry::window(
            "b".into(),
            "0".into(),
            "two".into(),
            "/b".into(),
            SortPriority::OtherSessionWindow,
            false,
        None,
        ),
        Entry::window(
            "b".into(),
            "1".into(),
            "logs".into(),
            "/b".into(),
            SortPriority::OtherSessionWindow,
            false,
        None,
        ),
        Entry::window(
            "a".into(),
            "0".into(),
            "one".into(),
            "/a".into(),
            SortPriority::OtherSessionWindow,
            false,
        None,
        ),
    ];
    state.replace_snapshot(snap_with(regrouped));

    let selected = state.selected_entry().expect("must remain actionable");
    assert_eq!(selected.target, "b:0");
}

/// Helper: render given state onto a 100x20 TestBackend and return all buffer content as a single string.
fn rendered_content(state: &AppState) -> String {
    let backend = TestBackend::new(100, 20);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|f| render(f, state)).unwrap();
    let buffer = terminal.backend().buffer();
    let mut out = String::new();
    for y in 0..20 {
        for x in 0..100 {
            out.push_str(buffer[(x, y)].symbol());
        }
        out.push('\n');
    }
    out
}

#[test]
fn preview_tmux_screen_renders_terminal_output() {
    let mut state = AppState::new(snap_with(vec![w("main")]));
    state.preview_visible = true;
    state.preview_state = PreviewState::TmuxScreen(TmuxScreenContent {
        session_name: "mysession".into(),
        path: "/home/user".into(),
        target: "mysession:0".into(),
        windows: vec!["🪟 main (active)".into()],
        screen_lines: vec![
            "user@host:~$ ls".into(),
            "file1.txt  file2.rs".into(),
            "user@host:~$ _".into(),
        ],
        is_fallback: false,
    });

    let content = rendered_content(&state);

    assert!(
        content.contains(''),
        "preview panel must have title with entry icon"
    );

    assert!(!content.contains("Session:"));
    assert!(!content.contains("Path:"));
    assert!(!content.contains("Status:"));
    assert!(!content.contains("Terminal Output"));

    // Pane content thực tế phải xuất hiện trong buffer
    assert!(
        content.contains("user@host"),
        "tmux screen must show pane content from terminal"
    );
    assert!(
        content.contains("file1.txt"),
        "tmux screen must show actual file listing from pane"
    );
}

#[test]
fn preview_tmux_screen_preserves_terminal_layout_without_metadata_block() {
    let mut state = AppState::new(snap_with(vec![w("main")]));
    state.preview_visible = true;
    state.preview_state = PreviewState::TmuxScreen(TmuxScreenContent {
        session_name: "mysession".into(),
        path: "/home/user".into(),
        target: "mysession:0".into(),
        windows: vec![],
        screen_lines: vec![
            "    indented-line".into(),
            "  ███ ui-block".into(),
            "prompt> _".into(),
        ],
        is_fallback: false,
    });

    let content = rendered_content(&state);

    // Tmux preview phải ưu tiên fidelity, không chen metadata block phía trên.
    assert!(!content.contains("Session:"));
    assert!(!content.contains("Path:"));
    assert!(!content.contains("Status:"));

    // Indentation/layout phải được giữ nguyên thay vì trim mất khoảng trắng đầu dòng.
    assert!(content.contains("    indented-line"));
    assert!(content.contains("  ███ ui-block"));
}

#[test]
fn preview_directory_listing_renders_entries() {
    let mut state = AppState::new(snap_with(vec![z("project")]));
    state.preview_visible = true;
    state.preview_state = PreviewState::DirectoryListing(DirectoryListingContent {
        name: "project".into(),
        path: "/home/user/project".into(),
        headline: "no tmux session".into(),
        entries: vec![
            "src/".into(),
            "Cargo.toml".into(),
            "README.md".into(),
            "target/".into(),
        ],
        has_session: false,
        source: "filesystem".into(),
    });

    let content = rendered_content(&state);

    assert!(
        content.contains(''),
        "preview panel must have title with directory icon for zoxide entry"
    );

    assert!(!content.is_empty(), "directory listing must have content");
    assert!(
        content.contains("src/"),
        "directory listing must show directory entries"
    );
}

#[test]
fn preview_tmux_fallback_renders_directory_listing() {
    let mut state = AppState::new(snap_with(vec![w("main")]));
    state.preview_visible = true;
    state.preview_state = PreviewState::DirectoryListing(DirectoryListingContent {
        name: "mysession".into(),
        path: "/home/user/project".into(),
        headline: "tmux capture failed".into(),
        entries: vec!["src/".into(), "Cargo.toml".into(), "README.md".into()],
        has_session: true,
        source: "tmux-fallback".into(),
    });

    let content = rendered_content(&state);

    assert!(
        content.contains(''),
        "preview panel must have title with entry icon"
    );

    assert!(
        content.contains("Cargo.toml"),
        "directory listing must show file entries"
    );
    assert!(
        content.contains("src"),
        "directory listing must show directory entries"
    );
}

#[test]
fn preview_fidelity_distinguishes_tmux_screen_and_directory_listing_metadata() {
    let mut state = AppState::new(snap_with(vec![w("main")]));
    state.preview_visible = true;

    state.preview_state = PreviewState::TmuxScreen(TmuxScreenContent {
        session_name: "s".into(),
        path: "/workspace".into(),
        target: "s:0".into(),
        windows: vec!["  ● 🪟 [0]: main".into()],
        screen_lines: vec!["prompt$ pwd".into(), "/workspace".into()],
        is_fallback: false,
    });
    let tmux_content = rendered_content(&state);
    assert!(!tmux_content.contains("Directory:"));
    assert!(!tmux_content.contains("Path:"));
    assert!(!tmux_content.contains("Status:"));
    assert!(tmux_content.contains("prompt$ pwd"));

    state.preview_state = PreviewState::DirectoryListing(DirectoryListingContent {
        name: "workspace".into(),
        path: "/workspace".into(),
        headline: String::new(),
        entries: vec!["drwxr-xr-x     -  Apr 04 12:00   src".into()],
        has_session: false,
        source: "tmux-fallback".into(),
    });
    let listing_content = rendered_content(&state);
    assert!(listing_content.contains("src"));
}

#[test]
fn preview_error_renders_clear_message() {
    let mut state = AppState::new(snap_with(vec![w("broken")]));
    state.preview_visible = true;
    state.preview_state = PreviewState::Error("connection refused".into());

    let content = rendered_content(&state);

    // Error branch phải hiện thị rõ ràng
    assert!(
        content.contains("Preview error"),
        "error state must show 'Preview error' header"
    );
    assert!(
        content.contains("connection refused"),
        "error state must include the original error message"
    );
}
