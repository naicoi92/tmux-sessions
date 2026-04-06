use crate::adapters::tmux::{FakeTmuxSource, TmuxAdapter, TmuxSource};
use crate::adapters::zoxide::{ZoxideAdapter, ZoxideSource};
use crate::app::tmux_window_mapper::map_raw_windows_to_entries;
use crate::domain::error::AdapterError;
use crate::domain::snapshot::Snapshot;
use crate::domain::sort::build_sorted_board;

const DEFAULT_ZOXIDE_LIMIT: usize = 100;

pub struct SnapshotLoader {
    tmux: Box<dyn TmuxSource>,
    zoxide: Box<dyn ZoxideSource>,
    zoxide_limit: usize,
}

impl SnapshotLoader {
    pub fn new(tmux: Box<dyn TmuxSource>, zoxide: Box<dyn ZoxideSource>) -> Self {
        Self {
            tmux,
            zoxide,
            zoxide_limit: DEFAULT_ZOXIDE_LIMIT,
        }
    }

    pub fn with_zoxide_limit(mut self, limit: usize) -> Self {
        self.zoxide_limit = limit;
        self
    }

    pub fn load(&self) -> Result<Snapshot, AdapterError> {
        // Graceful fallback: nếu tmux không available, dùng giá trị mặc định
        let current_session = self
            .tmux
            .current_session()
            .unwrap_or_else(|_| "default".into());
        let current_window_index = self
            .tmux
            .current_window_index()
            .unwrap_or_else(|_| "0".into());

        let raw_windows = self.tmux.list_windows().unwrap_or_default();
        let tmux_entries =
            map_raw_windows_to_entries(raw_windows, &current_session, &current_window_index);

        let zoxide_entries = self
            .zoxide
            .directories(self.zoxide_limit)
            .unwrap_or_default();

        let entries = build_sorted_board(
            &current_session,
            &current_window_index,
            tmux_entries,
            zoxide_entries,
        );

        let current_window = format!("{current_session}:{current_window_index}");
        Ok(Snapshot::new(entries, current_session, current_window))
    }
}

pub fn create_production_loader() -> SnapshotLoader {
    SnapshotLoader::new(Box::new(TmuxAdapter::new()), Box::new(ZoxideAdapter::new()))
}

pub fn create_debug_loader() -> SnapshotLoader {
    use crate::adapters::zoxide::FakeZoxideSource;
    SnapshotLoader::new(
        Box::new(FakeTmuxSource::new()),
        Box::new(FakeZoxideSource { paths: vec![] }),
    )
}

pub fn create_test_loader(
    tmux: Box<dyn TmuxSource>,
    zoxide: Box<dyn ZoxideSource>,
) -> SnapshotLoader {
    SnapshotLoader::new(tmux, zoxide)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapters::tmux::{FakeTmuxSource, RawWindow};
    use crate::adapters::zoxide::FakeZoxideSource;
    use crate::domain::entry::{EntryType, SortPriority};

    fn make_loader(
        windows: Vec<RawWindow>,
        sessions: Vec<crate::adapters::tmux::RawSession>,
        zoxide_paths: Vec<String>,
        current_session: &str,
        current_window_idx: &str,
    ) -> SnapshotLoader {
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

    fn w(session: &str, index: &str, name: &str) -> RawWindow {
        RawWindow {
            session_name: session.into(),
            window_index: index.into(),
            window_name: name.into(),
            window_path: format!("/{session}"),
        }
    }

    #[test]
    fn load_returns_snapshot_with_sorted_entries() {
        let loader = make_loader(
            vec![
                w("s2", "0", "remote"),
                w("s1", "1", "edit"),
                w("s1", "0", "main"),
            ],
            vec![],
            vec!["/proj".into()],
            "s1",
            "0",
        );
        let snap = loader.load().unwrap();

        assert_eq!(snap.len(), 4);
        assert_eq!(snap.current_session, "s1");
        assert_eq!(snap.current_window, "s1:0");
        assert_eq!(snap.entries[0].priority, SortPriority::CurrentWindow);
        assert_eq!(
            snap.entries[1].priority,
            SortPriority::CurrentSessionOtherWindow
        );
        assert_eq!(snap.entries[2].priority, SortPriority::OtherSessionWindow);
        assert_eq!(snap.entries[3].priority, SortPriority::ZoxideDirectory);
    }

    #[test]
    fn load_with_empty_zoxide() {
        let loader = make_loader(vec![w("s1", "0", "main")], vec![], vec![], "s1", "0");
        let snap = loader.load().unwrap();

        assert_eq!(snap.len(), 1);
        assert_eq!(snap.entries[0].entry_type, EntryType::Window);
    }

    #[test]
    fn load_with_no_tmux_windows_returns_only_zoxide() {
        let loader = make_loader(vec![], vec![], vec!["/a".into(), "/b".into()], "s1", "0");
        let snap = loader.load().unwrap();

        assert_eq!(snap.len(), 2);
        for entry in &snap.entries {
            assert_eq!(entry.entry_type, EntryType::Zoxide);
        }
    }

    #[test]
    fn load_with_empty_everything() {
        let loader = make_loader(vec![], vec![], vec![], "s1", "0");
        let snap = loader.load().unwrap();

        assert!(snap.is_empty());
        assert_eq!(snap.current_session, "s1");
    }

    #[test]
    fn zoxide_limit_is_respected() {
        let loader = make_loader(
            vec![],
            vec![],
            vec!["/a".into(), "/b".into(), "/c".into()],
            "s1",
            "0",
        )
        .with_zoxide_limit(2);
        let snap = loader.load().unwrap();

        assert_eq!(snap.len(), 2);
    }

    #[test]
    fn production_loader_creates_without_panic() {
        let _loader = create_production_loader();
    }

    #[test]
    fn current_window_field_format() {
        let loader = make_loader(
            vec![w("mysession", "3", "w")],
            vec![],
            vec![],
            "mysession",
            "3",
        );
        let snap = loader.load().unwrap();

        assert_eq!(snap.current_window, "mysession:3");
    }

    #[test]
    fn zoxide_entries_come_after_all_windows() {
        let loader = make_loader(
            vec![w("other", "0", "w"), w("s1", "0", "main")],
            vec![],
            vec!["/z1".into(), "/z2".into()],
            "s1",
            "0",
        );
        let snap = loader.load().unwrap();

        let first_zoxide_idx = snap
            .entries
            .iter()
            .position(|e| e.entry_type == EntryType::Zoxide)
            .unwrap();
        for entry in &snap.entries[..first_zoxide_idx] {
            assert_eq!(entry.entry_type, EntryType::Window);
        }
    }
}
