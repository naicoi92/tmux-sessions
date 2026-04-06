use crate::domain::entry::EntryType;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    Goto {
        target: String,
        path: String,
        entry_type: EntryType,
    },
    Kill {
        target: String,
        entry_type: EntryType,
    },
    TogglePreview,
    Reload,
    Quit,
}

impl Action {
    pub fn goto_window(target: String, path: String) -> Self {
        Self::Goto {
            target,
            path,
            entry_type: EntryType::Window,
        }
    }

    pub fn goto_zoxide(target: String, path: String) -> Self {
        Self::Goto {
            target,
            path,
            entry_type: EntryType::Zoxide,
        }
    }

    pub fn kill_window(target: String) -> Self {
        Self::Kill {
            target,
            entry_type: EntryType::Window,
        }
    }

    pub fn kill_zoxide(target: String) -> Self {
        Self::Kill {
            target,
            entry_type: EntryType::Zoxide,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn goto_window_has_correct_entry_type() {
        let action = Action::goto_window("s:1".into(), "/path".into());
        match action {
            Action::Goto { entry_type, .. } => assert_eq!(entry_type, EntryType::Window),
            _ => panic!("expected Goto"),
        }
    }

    #[test]
    fn goto_zoxide_has_correct_entry_type() {
        let action = Action::goto_zoxide("/home/proj".into(), "/home/proj".into());
        match action {
            Action::Goto { entry_type, .. } => assert_eq!(entry_type, EntryType::Zoxide),
            _ => panic!("expected Goto"),
        }
    }

    #[test]
    fn kill_window_variant() {
        let action = Action::kill_window("s:2".into());
        match action {
            Action::Kill { entry_type, .. } => assert_eq!(entry_type, EntryType::Window),
            _ => panic!("expected Kill"),
        }
    }

    #[test]
    fn kill_zoxide_variant() {
        let action = Action::kill_zoxide("myproject".into());
        match action {
            Action::Kill { entry_type, .. } => assert_eq!(entry_type, EntryType::Zoxide),
            _ => panic!("expected Kill"),
        }
    }

    #[test]
    fn utility_variants() {
        assert_eq!(Action::TogglePreview, Action::TogglePreview);
        assert_eq!(Action::Reload, Action::Reload);
        assert_eq!(Action::Quit, Action::Quit);
    }

    #[test]
    fn action_equality() {
        let a = Action::goto_window("s:1".into(), "/p".into());
        let b = Action::goto_window("s:1".into(), "/p".into());
        assert_eq!(a, b);
    }
}
