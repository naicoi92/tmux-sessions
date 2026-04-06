use crate::adapters::tmux::RawWindow;
use crate::domain::entry::{Entry, SortPriority};

pub fn map_raw_windows_to_entries(
    raw: Vec<RawWindow>,
    current_session: &str,
    current_window_index: &str,
) -> Vec<Entry> {
    raw.into_iter()
        .map(|w| {
            let is_current =
                w.session_name == current_session && w.window_index == current_window_index;
            Entry::window(
                w.session_name,
                w.window_index,
                w.window_name,
                w.window_path,
                if is_current {
                    SortPriority::CurrentWindow
                } else {
                    SortPriority::CurrentSessionOtherWindow
                },
                is_current,
            )
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn map_raw_windows_marks_current_and_keeps_non_current_priority() {
        let raw = vec![
            RawWindow {
                session_name: "s1".into(),
                window_index: "0".into(),
                window_name: "main".into(),
                window_path: "/home".into(),
            },
            RawWindow {
                session_name: "s2".into(),
                window_index: "1".into(),
                window_name: "other".into(),
                window_path: "/tmp".into(),
            },
        ];

        let entries = map_raw_windows_to_entries(raw, "s1", "0");

        assert_eq!(entries.len(), 2);
        assert!(entries[0].is_current);
        assert!(!entries[1].is_current);
        assert_eq!(entries[0].priority, SortPriority::CurrentWindow);
        assert_eq!(entries[1].priority, SortPriority::CurrentSessionOtherWindow);
    }
}
