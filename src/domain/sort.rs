use crate::domain::entry::{Entry, SortPriority};

pub fn sort_entries(entries: &mut [Entry]) {
    entries.sort_by(|a, b| a.priority.cmp(&b.priority));
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
        Entry::window(
            session.into(),
            index.into(),
            name.into(),
            "/path".into(),
            SortPriority::CurrentSessionOtherWindow,
            is_current,
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
            ),
            Entry::window(
                "s1".into(),
                "0".into(),
                "real-current".into(),
                "/s1".into(),
                SortPriority::CurrentSessionOtherWindow,
                true,
            ),
            Entry::window(
                "s2".into(),
                "0".into(),
                "other-session".into(),
                "/s2".into(),
                SortPriority::CurrentSessionOtherWindow,
                false,
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
            ),
            Entry::window(
                "s1".into(),
                "1".into(),
                "current".into(),
                "/s1".into(),
                SortPriority::CurrentWindow,
                true,
            ),
        ];

        let board = build_sorted_board("s1", "1", tmux, vec![]);

        let other = board
            .iter()
            .find(|e| e.target == "s2:3")
            .expect("other session entry must exist");
        assert_eq!(other.priority, SortPriority::OtherSessionWindow);
    }
}
