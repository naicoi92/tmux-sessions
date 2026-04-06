use std::cmp::Ordering;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntryType {
    Window,
    Zoxide,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum SortPriority {
    CurrentWindow = 0,
    CurrentSessionOtherWindow = 1,
    OtherSessionWindow = 2,
    ZoxideDirectory = 3,
}

#[derive(Debug, Clone)]
pub struct Entry {
    pub entry_type: EntryType,
    pub display: String,
    pub target: String,
    pub path: String,
    pub priority: SortPriority,
    pub session_name: Option<String>,
    pub is_current: bool,
    pub matched_indices: Vec<u32>,
}

impl Entry {
    pub fn window(
        session_name: String,
        window_index: String,
        window_name: String,
        path: String,
        priority: SortPriority,
        is_current: bool,
    ) -> Self {
        let marker = if is_current { "▸ " } else { "  " };
        let display = format!(
            "{marker}◆ {session} [{index}]: {name}",
            session = session_name,
            index = window_index,
            name = window_name,
        );
        let target = format!("{session_name}:{window_index}");
        Self {
            entry_type: EntryType::Window,
            display,
            target,
            path,
            priority,
            session_name: Some(session_name),
            is_current,
            matched_indices: Vec::new(),
        }
    }

    pub fn zoxide(dir_name: String, full_path: String) -> Self {
        let display = format!("▤ {dir_name}");
        let target = full_path.clone();
        Self {
            entry_type: EntryType::Zoxide,
            display,
            target,
            path: full_path,
            priority: SortPriority::ZoxideDirectory,
            session_name: None,
            is_current: false,
            matched_indices: Vec::new(),
        }
    }

    pub fn with_matched_indices(mut self, indices: Vec<u32>) -> Self {
        self.matched_indices = indices;
        self
    }
}

impl PartialEq for Entry {
    fn eq(&self, other: &Self) -> bool {
        self.target == other.target && self.entry_type == other.entry_type
    }
}

impl Eq for Entry {}

impl PartialOrd for Entry {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Entry {
    fn cmp(&self, other: &Self) -> Ordering {
        self.priority.cmp(&other.priority)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn window_entry_has_correct_target() {
        let entry = Entry::window(
            "mysession".into(),
            "1".into(),
            "editor".into(),
            "/home/user".into(),
            SortPriority::CurrentWindow,
            true,
        );
        assert_eq!(entry.target, "mysession:1");
        assert_eq!(entry.entry_type, EntryType::Window);
        assert!(entry.is_current);
    }

    #[test]
    fn window_entry_target_is_session_and_window_index_format() {
        let entry = Entry::window(
            "work-session".into(),
            "12".into(),
            "shell".into(),
            "/tmp".into(),
            SortPriority::CurrentSessionOtherWindow,
            false,
        );

        assert_eq!(entry.target, "work-session:12");
        assert!(entry.target.contains(':'));
        assert!(!entry.target.contains('.'));
    }

    #[test]
    fn current_window_has_marker() {
        let entry = Entry::window(
            "s".into(),
            "0".into(),
            "main".into(),
            "/path".into(),
            SortPriority::CurrentWindow,
            true,
        );
        assert!(entry.display.starts_with("▸"));
    }

    #[test]
    fn non_current_window_no_marker() {
        let entry = Entry::window(
            "s".into(),
            "1".into(),
            "other".into(),
            "/path".into(),
            SortPriority::CurrentSessionOtherWindow,
            false,
        );
        assert!(!entry.display.starts_with("▸"));
    }

    #[test]
    fn zoxide_entry_has_correct_fields() {
        let entry = Entry::zoxide("myproject".into(), "/home/user/myproject".into());
        assert_eq!(entry.entry_type, EntryType::Zoxide);
        assert_eq!(entry.target, "/home/user/myproject");
        assert_eq!(entry.priority, SortPriority::ZoxideDirectory);
        assert!(entry.session_name.is_none());
    }

    #[test]
    fn sort_order_respects_priority() {
        let current = Entry::window(
            "s".into(),
            "0".into(),
            "a".into(),
            "/".into(),
            SortPriority::CurrentWindow,
            true,
        );
        let other_session = Entry::window(
            "s2".into(),
            "0".into(),
            "a".into(),
            "/".into(),
            SortPriority::OtherSessionWindow,
            false,
        );
        let zoxide = Entry::zoxide("dir".into(), "/dir".into());

        let mut entries = [zoxide, other_session, current];
        entries.sort();

        assert_eq!(entries[0].priority, SortPriority::CurrentWindow);
        assert_eq!(entries[1].priority, SortPriority::OtherSessionWindow);
        assert_eq!(entries[2].priority, SortPriority::ZoxideDirectory);
    }
}
