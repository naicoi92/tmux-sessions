use tmux_sessions::app::state::AppState;
use tmux_sessions::domain::entry::{Entry, SortPriority};
use tmux_sessions::domain::snapshot::Snapshot;
use tmux_sessions::preview::ansi;
use tmux_sessions::preview::types::{DirectoryListingContent, PreviewState, TmuxScreenContent};

fn make_snap(entries: Vec<Entry>) -> Snapshot {
    Snapshot::new(entries, "s1".into(), "s1:0".into())
}

fn e_win(session: &str, index: &str, name: &str) -> Entry {
    Entry::window(session.into(), index.into(), name.into(), "/p".into(), SortPriority::OtherSessionWindow, false, None)
}

// =====================================================================
// Edge case 1: Preview hidden → selection changes → preview toggle back
// =====================================================================

#[test]
fn preview_toggle_hide_change_selection_toggle_show_correct_entry() {
    let mut state = AppState::new(make_snap(vec![
        e_win("s1", "0", "first"),
        e_win("s1", "1", "second"),
        e_win("s1", "2", "third"),
    ]));

    assert!(state.preview_visible);
    assert_eq!(state.selected_index, 0);

    state.toggle_preview();
    assert!(!state.preview_visible);

    state.move_selection_down();
    state.move_selection_down();
    assert_eq!(state.selected_index, 2);

    state.toggle_preview();
    assert!(state.preview_visible);
    assert_eq!(state.selected_index, 2);

    let entry = state.selected_entry().unwrap();
    assert_eq!(entry.target, "s1:2");
}

// =====================================================================
// Edge case 2: Kill removes item, clamp_selection handles focus
// =====================================================================

#[test]
fn clamp_selection_after_filter_reduces_count() {
    let mut state = AppState::new(make_snap(vec![
        e_win("s1", "0", "aaa"),
        e_win("s2", "0", "bbb"),
        e_win("s3", "0", "ccc"),
    ]));
    state.selected_index = 2;

    state.set_filter('b');
    state.clamp_selection();

    assert_eq!(state.filtered_entries().len(), 1);
    assert_eq!(state.selected_index, 0);
    assert!(state.selected_entry().is_some());
}

#[test]
fn clamp_selection_to_empty_does_not_panic() {
    let mut state = AppState::new(make_snap(vec![
        e_win("s1", "0", "aaa"),
        e_win("s2", "0", "bbb"),
    ]));
    state.selected_index = 1;

    state.set_filter('z');
    state.clamp_selection();

    assert_eq!(state.filtered_entries().len(), 0);
    assert_eq!(state.selected_index, 0);
    assert!(state.selected_entry().is_none());
    assert!(state.build_action().is_none());
    assert!(state.build_kill_action().is_none());
}

#[test]
fn clear_filter_after_clamp_restores_valid_selection() {
    let mut state = AppState::new(make_snap(vec![
        e_win("s1", "0", "aaa"),
        e_win("s2", "0", "bbb"),
        e_win("s3", "0", "ccc"),
    ]));
    state.selected_index = 2;
    state.set_filter('z');
    state.clamp_selection();
    assert!(state.selected_entry().is_none());

    state.clear_filter();
    assert_eq!(state.filtered_entries().len(), 3);
    assert_eq!(state.selected_index, 0);
    assert!(state.selected_entry().is_some());
}

// =====================================================================
// Edge case 3: ANSI-heavy pane content
// =====================================================================

#[test]
fn ansi_strip_nested_multiple_codes() {
    let input = "\x1b[1;31m\x1b[38;5;123mbold colored\x1b[0m\x1b[22m text";
    assert_eq!(ansi::strip_ansi(input), "bold colored text");
}

#[test]
fn ansi_strip_sgr_38_2_rgb_truecolor() {
    let input = "\x1b[38;2;255;128;0mtruecolor\x1b[0m rest";
    assert_eq!(ansi::strip_ansi(input), "truecolor rest");
}

#[test]
fn ansi_strip_cursor_and_screen_codes() {
    let input = "before\x1b[2J\x1b[H\x1b[?25hafter";
    assert_eq!(ansi::strip_ansi(input), "beforeafter");
}

#[test]
fn ansi_strip_osc_title() {
    let input = "\x1b]0;window title\x07content";
    assert_eq!(ansi::strip_ansi(input), "content");
}

#[test]
fn ansi_strip_mixed_real_world() {
    let input =
        "\x1b[?2004h\x1b[1;32muser@host\x1b[0m \x1b[1;34m~/project\x1b[0m $ \x1b[Kcargo build\n\
\x1b[1;33mwarning\x1b[0m: unused import\n\
\x1b[32m   Compiling myapp v0.1.0\x1b[0m\n\
\x1b[2J\x1b[H\x1b[1;32m    Finished\x1b[0m";
    let stripped = ansi::strip_ansi(input);
    assert!(!stripped.contains('\x1b'));
    assert!(stripped.contains("cargo build"));
    assert!(stripped.contains("warning"));
    assert!(stripped.contains("Compiling"));
    assert!(stripped.contains("Finished"));
}

// =====================================================================
// Edge case 4: Empty preview for valid target
// =====================================================================

#[test]
fn empty_preview_content_window_variant_renderable() {
    let pc = PreviewState::TmuxScreen(TmuxScreenContent {
        session_name: "s".into(),
        path: "/".into(),
        target: "s:0".into(),
        windows: vec![],
        screen_lines: vec![],
        is_fallback: false,
    });
    match pc {
        PreviewState::TmuxScreen(ref content) => {
            assert_eq!(content.session_name, "s");
            assert!(content.windows.is_empty());
            assert!(content.screen_lines.is_empty());
        }
        _ => panic!("expected TmuxScreen"),
    }
}

#[test]
fn empty_preview_content_directory_variant_renderable() {
    let pc = PreviewState::DirectoryListing(DirectoryListingContent {
        name: "project".into(),
        path: "/home/user/project".into(),
        headline: "Enter will create a new window in current session".into(),
        entries: vec![],
        has_session: false,
        source: "read_dir".into(),
    });
    match pc {
        PreviewState::DirectoryListing(ref content) => {
            assert_eq!(content.name, "project");
            assert!(content.entries.is_empty());
        }
        _ => panic!("expected DirectoryListing"),
    }
}

#[test]
fn preview_content_error_variant() {
    let pc = PreviewState::Error("cannot read directory".into());
    match pc {
        PreviewState::Error(msg) => assert_eq!(msg, "cannot read directory"),
        _ => panic!("expected Error"),
    }
}

#[test]
fn preview_content_loading_variant() {
    let pc = PreviewState::Loading;
    assert!(matches!(pc, PreviewState::Loading));
}

#[test]
fn preview_content_empty_variant() {
    let pc = PreviewState::Empty;
    assert!(matches!(pc, PreviewState::Empty));
}

// =====================================================================
// Edge case 5: Reload after kill preserving selection index
// =====================================================================

#[test]
fn selection_clamp_after_snapshot_shrink() {
    let mut state = AppState::new(make_snap(vec![
        e_win("s1", "0", "a"),
        e_win("s2", "0", "b"),
        e_win("s3", "0", "c"),
        e_win("s4", "0", "d"),
    ]));
    state.selected_index = 3;

    state.snapshot = make_snap(vec![e_win("s1", "0", "a"), e_win("s3", "0", "c")]);
    state.clamp_selection();

    let max = state.filtered_entries().len().saturating_sub(1);
    assert!(state.selected_index <= max);
}

// =====================================================================
// Edge case 6: Session/window/path names with spaces
// =====================================================================

#[test]
fn entry_with_spaces_in_window_name() {
    let entry = Entry::window("my session".into(), "0".into(), "my window name".into(), "/path with spaces".into(), SortPriority::CurrentWindow, true, None);
    assert_eq!(entry.target, "my session:0");
    assert_eq!(entry.path, "/path with spaces");
    assert!(entry.display.contains("my window name"));
    assert!(entry.display.contains("my session"));
}

#[test]
fn entry_with_spaces_in_zoxide_path() {
    let entry = Entry::zoxide("my project".into(), "/home/user/my project".into());
    assert_eq!(entry.target, "/home/user/my project");
    assert_eq!(entry.path, "/home/user/my project");
}

// =====================================================================
// Edge case 7: Very long preview content bounded
// =====================================================================

#[test]
fn ansi_strip_lines_respects_max_with_many_lines() {
    let lines: Vec<String> = (0..50).map(|i| format!("line {i}")).collect();
    let input = lines.join("\n");
    let result = ansi::strip_ansi_lines(&input, 5);
    assert_eq!(result.len(), 5);
    assert_eq!(result[0], "line 0");
    assert_eq!(result[4], "line 4");
}

#[test]
fn ansi_strip_filters_blank_lines() {
    let input = "a\n\n\n\nb\n\nc";
    let result = ansi::strip_ansi_lines(input, 10);
    assert_eq!(result, vec!["a", "b", "c"]);
}

// =====================================================================
// Edge case 8: Rapid selection changes — stale preview
// =====================================================================

#[test]
fn filter_with_spaces_in_window_name_works() {
    let mut state = AppState::new(make_snap(vec![
        Entry::window(
            "s".into(),
            "0".into(),
            "build step".into(),
            "/".into(),
            SortPriority::OtherSessionWindow,
            false,
        None,
        ),
        Entry::window(
            "s".into(),
            "1".into(),
            "deploy prod".into(),
            "/".into(),
            SortPriority::OtherSessionWindow,
            false,
        None,
        ),
    ]));
    assert_eq!(state.filtered_entries().len(), 2);

    state.set_filter('u');
    assert_eq!(state.filtered_entries().len(), 1);

    state.clear_filter();
    assert_eq!(state.filtered_entries().len(), 2);

    state.set_filter('r');
    assert_eq!(state.filtered_entries().len(), 1);
}

#[test]
fn selection_change_updates_target_for_preview_trigger() {
    let mut state = AppState::new(make_snap(vec![
        e_win("s1", "0", "a"),
        e_win("s2", "0", "b"),
    ]));

    let target_a = state.selected_entry().unwrap().target.clone();
    assert_eq!(target_a, "s1:0");

    state.move_selection_down();
    let target_b = state.selected_entry().unwrap().target.clone();
    assert_eq!(target_b, "s2:0");
    assert_ne!(target_a, target_b);
}

#[test]
fn reload_like_snapshot_replace_preserves_selected_target() {
    let mut state = AppState::new(make_snap(vec![
        e_win("s1", "0", "a"),
        e_win("s2", "0", "b"),
        e_win("s3", "0", "c"),
    ]));
    state.move_selection_down();
    assert_eq!(state.selected_entry().unwrap().target, "s2:0");

    let reloaded = Snapshot::new(
        vec![
            Entry::window(
                "s3".into(),
                "0".into(),
                "c".into(),
                "/p".into(),
                SortPriority::OtherSessionWindow,
                false,
            None,
            ),
            Entry::window(
                "s2".into(),
                "0".into(),
                "b".into(),
                "/p".into(),
                SortPriority::OtherSessionWindow,
                false,
            None,
            ),
            Entry::window(
                "s1".into(),
                "0".into(),
                "a".into(),
                "/p".into(),
                SortPriority::OtherSessionWindow,
                false,
            None,
            ),
        ],
        "s1".into(),
        "s1:0".into(),
    );

    state.replace_snapshot(reloaded);
    assert_eq!(state.selected_entry().unwrap().target, "s2:0");
}

// =====================================================================
// Bonus: Entry equality based on target + entry_type
// =====================================================================

#[test]
fn entry_equality_same_target_different_priority() {
    let a = Entry::window("s".into(), "0".into(), "main".into(), "/".into(), SortPriority::CurrentWindow, true, None);
    let b = Entry::window("s".into(), "0".into(), "main".into(), "/other".into(), SortPriority::OtherSessionWindow, false, None);
    assert_eq!(a, b);
}

#[test]
fn entry_inequality_different_target() {
    let a = e_win("s1", "0", "a");
    let b = e_win("s1", "1", "b");
    assert_ne!(a, b);
}

// =====================================================================
// Bonus: Empty snapshot edge cases
// =====================================================================

#[test]
fn empty_snapshot_no_action_possible() {
    let state = AppState::new(make_snap(vec![]));
    assert!(state.selected_entry().is_none());
    assert!(state.build_action().is_none());
    assert!(state.build_kill_action().is_none());
    assert_eq!(state.filtered_entries().len(), 0);
}

#[test]
fn single_entry_move_down_clamps() {
    let mut state = AppState::new(make_snap(vec![e_win("s1", "0", "only")]));
    state.move_selection_down();
    assert_eq!(state.selected_index, 0);
    state.move_selection_up();
    assert_eq!(state.selected_index, 0);
}
