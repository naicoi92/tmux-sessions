use tmux_sessions::adapters::tmux::{FakeTmuxSource, RawWindow};
use tmux_sessions::adapters::zoxide::FakeZoxideSource;
use tmux_sessions::app::loader::create_test_loader;
use tmux_sessions::app::tmux_window_mapper::map_raw_windows_to_entries;
use tmux_sessions::domain::entry::{EntryType, SortPriority};
use tmux_sessions::domain::sort::build_sorted_board;

fn w(session: &str, index: &str, name: &str) -> RawWindow {
    RawWindow {
        session_name: session.into(),
        window_index: index.into(),
        window_name: name.into(),
        window_path: format!("/{session}"),
    }
}

fn build_loader(
    windows: Vec<RawWindow>,
    zoxide_paths: Vec<String>,
    current_session: &str,
    current_window_idx: &str,
) -> tmux_sessions::app::loader::SnapshotLoader {
    let tmux = FakeTmuxSource {
        windows,
        sessions: vec![],
        current_session_name: current_session.into(),
        current_window_idx: current_window_idx.into(),
        existing_sessions: vec![current_session.into()],
        fail_on: vec![],
    };
    let zoxide = FakeZoxideSource {
        paths: zoxide_paths,
    };
    create_test_loader(Box::new(tmux), Box::new(zoxide))
}

#[test]
fn integration_snapshot_load_sort_order() {
    let loader = build_loader(
        vec![
            w("s3", "0", "far"),
            w("s2", "0", "other"),
            w("s1", "2", "third"),
            w("s1", "1", "second"),
            w("s1", "0", "first"),
        ],
        vec!["/alpha".into(), "/beta".into()],
        "s1",
        "1",
    );

    let snap = loader.load().unwrap();

    assert_eq!(snap.len(), 7);

    assert_eq!(snap.entries[0].priority, SortPriority::CurrentWindow);
    assert_eq!(snap.entries[0].target, "s1:1");

    let current_session_count = snap
        .entries
        .iter()
        .take_while(|e| e.priority <= SortPriority::CurrentSessionOtherWindow)
        .count();
    assert_eq!(current_session_count, 3, "s1 should have 3 windows");

    let other_session_count = snap
        .entries
        .iter()
        .skip(current_session_count)
        .take_while(|e| e.entry_type == EntryType::Window)
        .count();
    assert_eq!(other_session_count, 2, "s2+s3 should have 2 windows");

    let zoxide_count = snap
        .entries
        .iter()
        .filter(|e| e.entry_type == EntryType::Zoxide)
        .count();
    assert_eq!(zoxide_count, 2);
}

#[test]
fn integration_empty_tmux_returns_zoxide_only() {
    let loader = build_loader(
        vec![],
        vec!["/p1".into(), "/p2".into(), "/p3".into()],
        "s1",
        "0",
    );
    let snap = loader.load().unwrap();

    assert_eq!(snap.len(), 3);
    for entry in &snap.entries {
        assert_eq!(entry.entry_type, EntryType::Zoxide);
    }
}

#[test]
fn integration_empty_zoxide_returns_tmux_only() {
    let loader = build_loader(vec![w("s1", "0", "main")], vec![], "s1", "0");
    let snap = loader.load().unwrap();

    assert_eq!(snap.len(), 1);
    assert_eq!(snap.entries[0].entry_type, EntryType::Window);
    assert_eq!(snap.entries[0].priority, SortPriority::CurrentWindow);
}

#[test]
fn integration_completely_empty_returns_valid_snapshot() {
    let loader = build_loader(vec![], vec![], "default", "0");
    let snap = loader.load().unwrap();

    assert!(snap.is_empty());
    assert_eq!(snap.current_session, "default");
    assert_eq!(snap.current_window, "default:0");
}

#[test]
fn integration_zoxide_limit_truncates() {
    let loader = build_loader(
        vec![],
        vec!["/a".into(), "/b".into(), "/c".into(), "/d".into()],
        "s1",
        "0",
    )
    .with_zoxide_limit(2);

    let snap = loader.load().unwrap();
    assert_eq!(snap.len(), 2);
}

#[test]
fn integration_single_session_multiple_windows_sorted() {
    let loader = build_loader(
        vec![
            w("only", "5", "last"),
            w("only", "2", "middle"),
            w("only", "0", "first"),
        ],
        vec![],
        "only",
        "2",
    );

    let snap = loader.load().unwrap();
    assert_eq!(snap.len(), 3);

    assert_eq!(snap.entries[0].priority, SortPriority::CurrentWindow);
    assert_eq!(snap.entries[0].target, "only:2");

    for entry in &snap.entries {
        assert_eq!(entry.session_name.as_deref(), Some("only"));
    }
}

#[test]
fn integration_many_sessions_current_session_first() {
    let loader = build_loader(
        vec![
            w("s5", "0", "a"),
            w("s4", "0", "a"),
            w("s3", "0", "a"),
            w("s2", "0", "a"),
            w("s1", "1", "b"),
            w("s1", "0", "a"),
        ],
        vec![],
        "s3",
        "0",
    );

    let snap = loader.load().unwrap();
    assert_eq!(snap.entries[0].target, "s3:0");
    assert_eq!(snap.entries[0].priority, SortPriority::CurrentWindow);

    for entry in &snap.entries {
        if entry.priority == SortPriority::CurrentSessionOtherWindow {
            assert_eq!(
                entry.session_name.as_deref(),
                Some("s3"),
                "only s3 windows should have CurrentSessionOtherWindow priority"
            );
        }
    }
}

#[test]
fn integration_current_window_string_uses_session_colon_index_format() {
    let loader = build_loader(vec![w("work", "12", "editor")], vec![], "work", "12");

    let snap = loader.load().unwrap();
    assert_eq!(snap.current_window, "work:12");
}

#[test]
fn integration_other_session_priority_comes_from_board_normalization() {
    let mapped = map_raw_windows_to_entries(
        vec![w("s2", "3", "foreign"), w("s1", "1", "current")],
        "s1",
        "1",
    );

    let s2_before = mapped
        .iter()
        .find(|e| e.target == "s2:3")
        .expect("s2 window should exist before sort board");
    assert_eq!(
        s2_before.priority,
        SortPriority::CurrentSessionOtherWindow,
        "windows_to_entries should not assign OtherSessionWindow directly"
    );

    let board = build_sorted_board("s1", "1", mapped, vec![]);
    let s2_after = board
        .iter()
        .find(|e| e.target == "s2:3")
        .expect("s2 window should exist after sort board");

    assert_eq!(s2_after.priority, SortPriority::OtherSessionWindow);
}
