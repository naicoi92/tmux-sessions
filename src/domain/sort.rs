use crate::domain::entry::{Entry, SortPriority};
use std::cmp::Ordering;
use std::collections::HashMap;

pub fn sort_entries(entries: &mut [Entry]) {
    let mut session_max_activity: HashMap<String, Option<i64>> = HashMap::new();
    let mut session_first_index: HashMap<String, usize> = HashMap::new();

    for (idx, entry) in entries.iter().enumerate() {
        if entry.priority != SortPriority::OtherSessionWindow {
            continue;
        }

        let Some(session) = entry.session_name.as_ref() else {
            continue;
        };

        session_first_index.entry(session.clone()).or_insert(idx);

        let max_slot = session_max_activity.entry(session.clone()).or_insert(None);
        *max_slot = max_activity(*max_slot, entry.window_activity);
    }

    entries.sort_by(|a, b| {
        let priority_cmp = a.priority.cmp(&b.priority);
        if priority_cmp != Ordering::Equal {
            return priority_cmp;
        }

        match a.priority {
            SortPriority::CurrentWindow | SortPriority::ZoxideDirectory => Ordering::Equal,
            SortPriority::CurrentSessionOtherWindow => {
                compare_activity_desc(a.window_activity, b.window_activity)
            }
            SortPriority::OtherSessionWindow => {
                let a_session = a.session_name.as_deref();
                let b_session = b.session_name.as_deref();

                if a_session == b_session {
                    return compare_activity_desc(a.window_activity, b.window_activity);
                }

                let a_session_max = a_session
                    .and_then(|s| session_max_activity.get(s).copied())
                    .unwrap_or(None);
                let b_session_max = b_session
                    .and_then(|s| session_max_activity.get(s).copied())
                    .unwrap_or(None);

                let session_activity_cmp = compare_activity_desc(a_session_max, b_session_max);
                if session_activity_cmp != Ordering::Equal {
                    return session_activity_cmp;
                }

                let a_first_idx = a_session
                    .and_then(|s| session_first_index.get(s))
                    .copied()
                    .unwrap_or(usize::MAX);
                let b_first_idx = b_session
                    .and_then(|s| session_first_index.get(s))
                    .copied()
                    .unwrap_or(usize::MAX);

                a_first_idx.cmp(&b_first_idx)
            }
        }
    });
}

fn compare_activity_desc(a: Option<i64>, b: Option<i64>) -> Ordering {
    match (a, b) {
        (Some(a_ts), Some(b_ts)) => b_ts.cmp(&a_ts),
        (Some(_), None) => Ordering::Less,
        (None, Some(_)) => Ordering::Greater,
        (None, None) => Ordering::Equal,
    }
}

fn max_activity(a: Option<i64>, b: Option<i64>) -> Option<i64> {
    match (a, b) {
        (Some(a_ts), Some(b_ts)) => Some(a_ts.max(b_ts)),
        (Some(a_ts), None) => Some(a_ts),
        (None, Some(b_ts)) => Some(b_ts),
        (None, None) => None,
    }
}

pub fn build_sorted_board(
    current_session: &str,
    _current_window_index: &str,
    tmux_entries: Vec<Entry>,
    zoxide_entries: Vec<Entry>,
) -> Vec<Entry> {
    let mut board: Vec<Entry> = tmux_entries
        .into_iter()
        .map(|mut e| {
            match (&e.session_name.as_deref(), e.priority) {
                (Some(s), SortPriority::CurrentWindow)
                | (Some(s), SortPriority::CurrentSessionOtherWindow)
                    if *s == current_session && e.is_current =>
                {
                    e.priority = SortPriority::CurrentWindow;
                }
                (Some(s), _) if *s == current_session => {
                    e.priority = SortPriority::CurrentSessionOtherWindow;
                }
                (Some(_), _) => {
                    e.priority = SortPriority::OtherSessionWindow;
                }
                _ => {}
            }
            e
        })
        .chain(zoxide_entries)
        .collect();

    sort_entries(&mut board);
    board
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::entry::EntryType;

    fn make_window(session: &str, index: &str, name: &str, is_current: bool) -> Entry {
        make_window_with_activity(session, index, name, is_current, None)
    }

    fn make_window_with_activity(
        session: &str,
        index: &str,
        name: &str,
        is_current: bool,
        window_activity: Option<i64>,
    ) -> Entry {
        Entry::window(
            session.into(),
            index.into(),
            name.into(),
            "/path".into(),
            SortPriority::CurrentSessionOtherWindow,
            is_current,
            window_activity,
        )
    }

    #[test]
    fn sort_puts_current_window_first() {
        let entries = vec![
            make_window("s2", "0", "a", false),
            make_window("s1", "1", "b", false),
            make_window("s1", "0", "a", true),
        ];
        let board = build_sorted_board("s1", "0", entries, vec![]);

        let current_session_count = board
            .iter()
            .filter(|e| e.session_name.as_deref() == Some("s1"))
            .count();
        let other_session_count = board.len() - current_session_count;

        assert_eq!(current_session_count, 2);
        assert_eq!(other_session_count, 1);

        for (i, entry) in board.iter().enumerate().take(current_session_count) {
            assert_eq!(
                entry.session_name.as_deref(),
                Some("s1"),
                "position {i} should be current session"
            );
        }
    }

    #[test]
    fn zoxide_entries_after_all_tmux() {
        let tmux = vec![make_window("s1", "0", "a", true)];
        let zoxide = vec![
            Entry::zoxide("alpha".into(), "/alpha".into()),
            Entry::zoxide("beta".into(), "/beta".into()),
        ];
        let board = build_sorted_board("s1", "0", tmux, zoxide);

        assert_eq!(board[0].entry_type, EntryType::Window);
        assert_eq!(board[1].entry_type, EntryType::Zoxide);
        assert_eq!(board[2].entry_type, EntryType::Zoxide);
    }

    #[test]
    fn full_priority_order_current_other_session_zoxide() {
        let tmux = vec![
            make_window("s2", "0", "x", false),
            make_window("s1", "1", "b", false),
            make_window("s1", "0", "a", true),
        ];
        let zoxide = vec![Entry::zoxide("dir".into(), "/dir".into())];
        let board = build_sorted_board("s1", "0", tmux, zoxide);

        assert_eq!(board[0].priority, SortPriority::CurrentWindow);
        assert_eq!(board[1].priority, SortPriority::CurrentSessionOtherWindow);
        assert_eq!(board[2].priority, SortPriority::OtherSessionWindow);
        assert_eq!(board[3].priority, SortPriority::ZoxideDirectory);
    }

    #[test]
    fn empty_inputs_produce_empty_board() {
        let board = build_sorted_board("s1", "0", vec![], vec![]);
        assert!(board.is_empty());
    }

    #[test]
    fn only_zoxide_entries() {
        let zoxide = vec![Entry::zoxide("a".into(), "/a".into())];
        let board = build_sorted_board("s1", "0", vec![], zoxide);
        assert_eq!(board.len(), 1);
        assert_eq!(board[0].priority, SortPriority::ZoxideDirectory);
    }

    #[test]
    fn normalization_promotes_only_real_current_window() {
        let tmux = vec![
            Entry::window(
                "s1".into(),
                "5".into(),
                "wrong-flag".into(),
                "/s1".into(),
                SortPriority::CurrentWindow,
                false,
                None,
            ),
            Entry::window(
                "s1".into(),
                "0".into(),
                "real-current".into(),
                "/s1".into(),
                SortPriority::CurrentSessionOtherWindow,
                true,
                None,
            ),
            Entry::window(
                "s2".into(),
                "0".into(),
                "other-session".into(),
                "/s2".into(),
                SortPriority::CurrentSessionOtherWindow,
                false,
                None,
            ),
        ];

        let board = build_sorted_board("s1", "0", tmux, vec![]);

        assert_eq!(board[0].target, "s1:0");
        assert_eq!(board[0].priority, SortPriority::CurrentWindow);
        assert_eq!(board[1].target, "s1:5");
        assert_eq!(board[1].priority, SortPriority::CurrentSessionOtherWindow);
        assert_eq!(board[2].target, "s2:0");
        assert_eq!(board[2].priority, SortPriority::OtherSessionWindow);
    }

    #[test]
    fn normalization_assigns_other_session_after_tmux_mapping_style_input() {
        let tmux = vec![
            Entry::window(
                "s2".into(),
                "3".into(),
                "other".into(),
                "/s2".into(),
                SortPriority::CurrentSessionOtherWindow,
                false,
                None,
            ),
            Entry::window(
                "s1".into(),
                "1".into(),
                "current".into(),
                "/s1".into(),
                SortPriority::CurrentWindow,
                true,
                None,
            ),
        ];

        let board = build_sorted_board("s1", "1", tmux, vec![]);

        let other = board
            .iter()
            .find(|e| e.target == "s2:3")
            .expect("other session entry must exist");
        assert_eq!(other.priority, SortPriority::OtherSessionWindow);
    }

    #[test]
    fn sort_orders_windows_by_recent_activity_within_priority_buckets() {
        let tmux = vec![
            make_window_with_activity("s1", "1", "older", false, Some(10)),
            make_window_with_activity("s1", "0", "current", true, Some(100)),
            make_window_with_activity("s1", "2", "newer", false, Some(90)),
            make_window_with_activity("s2", "0", "other-old", false, Some(1)),
            make_window_with_activity("s2", "1", "other-new", false, Some(50)),
        ];

        let board = build_sorted_board("s1", "0", tmux, vec![]);
        let targets: Vec<&str> = board.iter().map(|e| e.target.as_str()).collect();

        assert_eq!(targets, vec!["s1:0", "s1:2", "s1:1", "s2:1", "s2:0"]);
    }

    #[test]
    fn sort_preserves_relative_order_when_activity_missing_or_tied() {
        let tmux = vec![
            make_window_with_activity("s1", "0", "current", true, Some(999)),
            make_window_with_activity("s1", "3", "tie-a", false, Some(50)),
            make_window_with_activity("s1", "4", "tie-b", false, Some(50)),
            make_window_with_activity("s1", "5", "none-a", false, None),
            make_window_with_activity("s1", "6", "none-b", false, None),
            make_window_with_activity("s2", "0", "other-none-a", false, None),
            make_window_with_activity("s3", "0", "other-none-b", false, None),
        ];

        let board = build_sorted_board("s1", "0", tmux, vec![]);
        let targets: Vec<&str> = board.iter().map(|e| e.target.as_str()).collect();

        assert_eq!(
            targets,
            vec!["s1:0", "s1:3", "s1:4", "s1:5", "s1:6", "s2:0", "s3:0"]
        );
    }

    #[test]
    fn sort_orders_other_sessions_by_session_max_activity() {
        let tmux = vec![
            make_window_with_activity("s1", "0", "current", true, Some(80)),
            make_window_with_activity("s2", "1", "s2-max-10", false, Some(10)),
            make_window_with_activity("s3", "0", "s3-max-70", false, Some(70)),
            make_window_with_activity("s2", "0", "s2-older", false, Some(5)),
            make_window_with_activity("s4", "0", "s4-none", false, None),
            make_window_with_activity("s3", "1", "s3-older", false, Some(1)),
        ];

        let board = build_sorted_board("s1", "0", tmux, vec![]);
        let targets: Vec<&str> = board.iter().map(|e| e.target.as_str()).collect();

        assert_eq!(
            targets,
            vec!["s1:0", "s3:0", "s3:1", "s2:1", "s2:0", "s4:0"]
        );
    }
}
