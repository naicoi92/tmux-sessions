use std::time::{Duration, Instant};

use tmux_sessions::adapters::tmux::{FakeTmuxSource, RawWindow};
use tmux_sessions::adapters::zoxide::FakeZoxideSource;
use tmux_sessions::app::loader::create_test_loader;
use tmux_sessions::domain::entry::SortPriority;

fn make_loaded_loader(
    window_count: usize,
    zoxide_count: usize,
) -> tmux_sessions::app::loader::SnapshotLoader {
    let windows: Vec<RawWindow> = (0..window_count)
        .map(|i| RawWindow {
            session_name: format!("s{}", i % 3),
            window_index: format!("{i}"),
            window_name: format!("w{i}"),
            window_path: format!("/path/{i}"),
            window_activity: None,
        })
        .collect();

    let zoxide: Vec<String> = (0..zoxide_count)
        .map(|i| format!("/home/user/project{i}"))
        .collect();

    let tmux = FakeTmuxSource {
        windows,
        sessions: vec![],
        current_session_name: "s0".into(),
        current_window_idx: "0".into(),
        existing_sessions: vec!["s0".into(), "s1".into(), "s2".into()],
        fail_on: vec![],
    };
    let zoxide_src = FakeZoxideSource { paths: zoxide };
    create_test_loader(Box::new(tmux), Box::new(zoxide_src))
}

#[test]
fn snapshot_load_small_under_50ms() {
    let loader = make_loaded_loader(5, 5);
    let start = Instant::now();
    let result = loader.load();
    let elapsed = start.elapsed();
    assert!(result.is_ok());
    assert!(
        elapsed < Duration::from_millis(50),
        "small load took {:?}",
        elapsed
    );
}

#[test]
fn snapshot_load_medium_under_100ms() {
    let loader = make_loaded_loader(50, 50);
    let start = Instant::now();
    let result = loader.load();
    let elapsed = start.elapsed();
    assert!(result.is_ok());
    assert!(
        elapsed < Duration::from_millis(100),
        "medium load took {:?}",
        elapsed
    );
}

#[test]
fn snapshot_load_large_under_200ms() {
    let loader = make_loaded_loader(200, 100);
    let start = Instant::now();
    let result = loader.load();
    let elapsed = start.elapsed();
    assert!(result.is_ok());
    assert!(
        elapsed < Duration::from_millis(200),
        "large load took {:?}",
        elapsed
    );
}

#[test]
fn filtered_entries_fast_on_snapshot() {
    use tmux_sessions::app::state::AppState;
    use tmux_sessions::domain::entry::Entry;
    use tmux_sessions::domain::snapshot::Snapshot;

    let entries: Vec<Entry> = (0..100)
        .map(|i| {
            Entry::window(
                format!("s{}", i % 5),
                format!("{i}"),
                format!("window-{i}"),
                format!("/path/{i}"),
                SortPriority::OtherSessionWindow,
                false,
            )
        })
        .collect();
    let snap = Snapshot::new(entries, "s0".into(), "s0:0".into());
    let state = AppState::new(snap);

    let start = Instant::now();
    for _ in 0..1000 {
        let _ = state.filtered_entries();
    }
    let elapsed = start.elapsed();
    assert!(
        elapsed < Duration::from_millis(50),
        "1000x filtered_entries took {:?}",
        elapsed
    );
}

#[test]
fn snapshot_load_empty_is_instant() {
    let loader = make_loaded_loader(0, 0);
    let start = Instant::now();
    let result = loader.load();
    let elapsed = start.elapsed();
    assert!(result.is_ok());
    assert!(
        elapsed < Duration::from_millis(10),
        "empty load took {:?}",
        elapsed
    );
}
