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
        window_activity: None,
    }
}

fn wa(session: &str, index: &str, name: &str, activity: Option<i64>) -> RawWindow {
    RawWindow {
        session_name: session.into(),
        window_index: index.into(),
        window_name: name.into(),
        window_path: format!("/{session}"),
        window_activity: activity,
    }
}

fn build_loader(
    windows: Vec<RawWindow>,
    zoxide_paths: Vec<String>,
    current_session: &str,
    current_window_idx: &str,
) -> tmux_sessions::app::loader::SnapshotLoader {
    use std::collections::HashMap;
    use tmux_sessions::adapters::tmux::RawSession;

    // Build session_activity from max window_activity per session
    let mut max_activity: HashMap<String, Option<i64>> = HashMap::new();
    for w in &windows {
        let entry = max_activity.entry(w.session_name.clone()).or_insert(None);
        match (*entry, w.window_activity) {
            (Some(a), Some(b)) => *entry = Some(a.max(b)),
            (None, Some(b)) => *entry = Some(b),
            _ => {}
        }
    }

    let sessions: Vec<RawSession> = max_activity
        .keys()
        .map(|name| RawSession {
            session_name: name.clone(),
            attached: name == current_session,
            session_activity: max_activity[name],
        })
        .collect();

    let tmux = FakeTmuxSource {
        windows,
        sessions,
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
    // s1 is current session (current window idx 1)
    // s2 has max activity 200, s3 has max activity 50
    // Within current session: activity 90 > 10
    let loader = build_loader(
        vec![
            wa("s3", "0", "idle", Some(50)),
            wa("s2", "1", "recent", Some(200)),
            wa("s2", "0", "old", Some(30)),
            wa("s1", "2", "third", Some(10)),
            wa("s1", "1", "second", Some(90)),
            wa("s1", "0", "first", Some(20)),
        ],
        vec!["/alpha".into(), "/beta".into()],
        "s1",
        "1",
    );

    let snap = loader.load().unwrap();
    assert_eq!(snap.len(), 8);

    // 1. Current window first
    assert_eq!(snap.entries[0].priority, SortPriority::CurrentWindow);
    assert_eq!(snap.entries[0].target, "s1:1");

    // 2. Current session windows sorted by activity desc (90 > 20 > 10)
    let current_session: Vec<_> = snap
        .entries
        .iter()
        .filter(|e| e.priority == SortPriority::CurrentSessionOtherWindow)
        .collect();
    assert_eq!(current_session.len(), 2, "s1 should have 2 other windows");
    assert_eq!(current_session[0].target, "s1:0"); // activity 20
    assert_eq!(current_session[1].target, "s1:2"); // activity 10
                                                   // s1:0 (20) > s1:2 (10) — desc by activity

    // 3. Other sessions: s2 (max 200) before s3 (max 50)
    let other_session: Vec<_> = snap
        .entries
        .iter()
        .filter(|e| e.priority == SortPriority::OtherSessionWindow)
        .collect();
    assert_eq!(other_session.len(), 3, "s2+s3 should have 3 windows");
    // s2 windows come first (max activity 200 > 50)
    assert_eq!(other_session[0].session_name.as_deref(), Some("s2"));
    assert_eq!(other_session[1].session_name.as_deref(), Some("s2"));
    // Within s2: activity 200 > 30
    assert_eq!(other_session[0].target, "s2:1"); // activity 200
    assert_eq!(other_session[1].target, "s2:0"); // activity 30
                                                 // Then s3
    assert_eq!(other_session[2].session_name.as_deref(), Some("s3"));
    assert_eq!(other_session[2].target, "s3:0"); // activity 50

    // 4. Zoxide after all tmux
    let zoxide_count = snap
        .entries
        .iter()
        .filter(|e| e.entry_type == EntryType::Zoxide)
        .count();
    assert_eq!(zoxide_count, 2);
    let first_zoxide_idx = snap
        .entries
        .iter()
        .position(|e| e.entry_type == EntryType::Zoxide)
        .unwrap();
    assert_eq!(
        first_zoxide_idx, 6,
        "zoxide starts after 3 s1 + 3 other windows"
    );
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
        &std::collections::HashMap::new(),
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

    let board = build_sorted_board("s1", "1", mapped, vec![], &std::collections::HashMap::new());
    let s2_after = board
        .iter()
        .find(|e| e.target == "s2:3")
        .expect("s2 window should exist after sort board");

    assert_eq!(s2_after.priority, SortPriority::OtherSessionWindow);
}

#[test]
fn integration_snapshot_load_orders_sessions_by_max_window_activity() {
    // s1 is current (window idx 0, activity 100)
    // s2 max activity = 500, s3 max activity = 300, s4 max activity = 10
    let loader = build_loader(
        vec![
            wa("s4", "0", "stale", Some(10)),
            wa("s3", "1", "s3-active", Some(300)),
            wa("s3", "0", "s3-older", Some(50)),
            wa("s2", "0", "s2-max", Some(500)),
            wa("s2", "1", "s2-mid", Some(200)),
            wa("s2", "2", "s2-low", Some(100)),
            wa("s1", "0", "current", Some(100)),
        ],
        vec!["/zdir".into()],
        "s1",
        "0",
    );

    let snap = loader.load().unwrap();
    assert_eq!(snap.len(), 8);

    assert_eq!(snap.entries[0].target, "s1:0");
    assert_eq!(snap.entries[0].priority, SortPriority::CurrentWindow);

    let other_targets: Vec<String> = snap
        .entries
        .iter()
        .filter(|e| e.priority == SortPriority::OtherSessionWindow)
        .map(|e| e.target.clone())
        .collect();
    assert_eq!(
        other_targets,
        vec!["s2:0", "s2:1", "s2:2", "s3:1", "s3:0", "s4:0"],
        "sessions ordered by max activity: s2(500) > s3(300) > s4(10), windows within by activity desc"
    );

    assert_eq!(snap.entries[7].entry_type, EntryType::Zoxide);
}

#[test]
fn integration_missing_activity_falls_back_without_losing_entries() {
    // s1 current, s2 has mixed activity (some None), s3 all None
    let loader = build_loader(
        vec![
            wa("s3", "0", "no-act-a", None),
            wa("s3", "1", "no-act-b", None),
            wa("s2", "0", "s2-with-act", Some(400)),
            wa("s2", "1", "s2-no-act", None),
            wa("s1", "0", "current", Some(999)),
        ],
        vec!["/fallback".into()],
        "s1",
        "0",
    );

    let snap = loader.load().unwrap();
    assert_eq!(snap.len(), 6, "all 5 windows + 1 zoxide present");

    assert_eq!(snap.entries[0].target, "s1:0");

    let other: Vec<_> = snap
        .entries
        .iter()
        .filter(|e| e.priority == SortPriority::OtherSessionWindow)
        .collect();
    assert_eq!(other.len(), 4, "s2 + s3 = 4 windows");

    // s2 (max 400) before s3 (all None)
    assert_eq!(other[0].session_name.as_deref(), Some("s2"));
    assert_eq!(other[1].session_name.as_deref(), Some("s2"));
    assert_eq!(other[2].session_name.as_deref(), Some("s3"));
    assert_eq!(other[3].session_name.as_deref(), Some("s3"));

    // Within s2: Some(400) before None
    assert_eq!(other[0].target, "s2:0");
    assert_eq!(other[1].target, "s2:1");

    assert_eq!(snap.entries[5].entry_type, EntryType::Zoxide);
}
