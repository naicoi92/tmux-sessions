use crate::adapters::tmux::RawWindow;
use crate::domain::entry::{Entry, SortPriority};
use std::collections::HashMap;

pub fn map_raw_windows_to_entries(
    raw: Vec<RawWindow>,
    current_session: &str,
    current_window_index: &str,
    session_activities: &HashMap<String, Option<i64>>,
) -> Vec<Entry> {
    raw.into_iter()
        .map(|w| {
            let session_activity = session_activities.get(&w.session_name).copied().flatten();
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
                w.window_activity,
                session_activity,
            )
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn map_raw_windows_marks_current_and_keeps_non_current_priority() {
        let raw = vec![
            RawWindow {
                session_name: "s1".into(),
                window_index: "0".into(),
                window_name: "main".into(),
                window_path: "/home".into(),
                window_activity: None,
            },
            RawWindow {
                session_name: "s2".into(),
                window_index: "1".into(),
                window_name: "other".into(),
                window_path: "/tmp".into(),
                window_activity: None,
            },
        ];

        let entries = map_raw_windows_to_entries(raw, "s1", "0", &HashMap::new());

        assert_eq!(entries.len(), 2);
        assert!(entries[0].is_current);
        assert!(!entries[1].is_current);
        assert_eq!(entries[0].priority, SortPriority::CurrentWindow);
        assert_eq!(entries[1].priority, SortPriority::CurrentSessionOtherWindow);
    }

    #[test]
    fn map_raw_windows_propagates_window_activity() {
        let raw = vec![
            RawWindow {
                session_name: "s1".into(),
                window_index: "0".into(),
                window_name: "main".into(),
                window_path: "/home".into(),
                window_activity: Some(1714000000),
            },
            RawWindow {
                session_name: "s2".into(),
                window_index: "1".into(),
                window_name: "other".into(),
                window_path: "/tmp".into(),
                window_activity: None,
            },
            RawWindow {
                session_name: "s3".into(),
                window_index: "0".into(),
                window_name: "idle".into(),
                window_path: "/var".into(),
                window_activity: Some(1713000000),
            },
        ];

        let entries = map_raw_windows_to_entries(raw, "s1", "0", &HashMap::new());

        assert_eq!(entries.len(), 3);
        assert_eq!(entries[0].window_activity, Some(1714000000));
        assert_eq!(entries[1].window_activity, None);
        assert_eq!(entries[2].window_activity, Some(1713000000));
    }

    #[test]
    fn map_raw_windows_all_none_activity() {
        let raw = vec![
            RawWindow {
                session_name: "s1".into(),
                window_index: "0".into(),
                window_name: "a".into(),
                window_path: "/a".into(),
                window_activity: None,
            },
            RawWindow {
                session_name: "s1".into(),
                window_index: "1".into(),
                window_name: "b".into(),
                window_path: "/b".into(),
                window_activity: None,
            },
        ];

        let entries = map_raw_windows_to_entries(raw, "s1", "0", &HashMap::new());

        assert!(entries.iter().all(|e| e.window_activity.is_none()));
    }

    #[test]
    fn map_raw_windows_propagates_session_activity_by_session_name() {
        let raw = vec![RawWindow {
            session_name: "s2".into(),
            window_index: "1".into(),
            window_name: "other".into(),
            window_path: "/tmp".into(),
            window_activity: Some(111),
        }];
        let mut activities = HashMap::new();
        activities.insert("s2".to_string(), Some(1714000123));

        let entries = map_raw_windows_to_entries(raw, "s1", "0", &activities);

        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].session_activity, Some(1714000123));
    }
}
