use crate::domain::entry::Entry;

pub struct Snapshot {
    pub entries: Vec<Entry>,
    pub current_session: String,
    pub current_window: String,
}

impl Snapshot {
    pub fn new(entries: Vec<Entry>, current_session: String, current_window: String) -> Self {
        Self {
            entries,
            current_session,
            current_window,
        }
    }

    pub fn empty() -> Self {
        Self {
            entries: vec![],
            current_session: "default".into(),
            current_window: "default:0".into(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::entry::SortPriority;

    fn dummy_entry() -> Entry {
        Entry::window(
            "s".into(),
            "0".into(),
            "a".into(),
            "/".into(),
            SortPriority::CurrentWindow,
            true,
        )
    }

    #[test]
    fn new_snapshot() {
        let snap = Snapshot::new(vec![dummy_entry()], "session1".into(), "session1:0".into());
        assert_eq!(snap.current_session, "session1");
        assert_eq!(snap.len(), 1);
        assert!(!snap.is_empty());
    }

    #[test]
    fn empty_snapshot() {
        let snap = Snapshot::new(vec![], "s".into(), "s:0".into());
        assert!(snap.is_empty());
        assert_eq!(snap.len(), 0);
    }
}
