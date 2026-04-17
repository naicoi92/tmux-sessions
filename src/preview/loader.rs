use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use crate::domain::entry::Entry;
use crate::preview::generator::PreviewGenerator;
use crate::preview::types::{next_id, PreviewState};

const PREVIEW_TIMEOUT: Duration = Duration::from_secs(2);

pub struct PreviewResult {
    pub id: u64,
    pub target: String,
    pub dimensions: Option<(u16, u16)>,
    pub content: PreviewState,
}

/// Spawns a background thread per preview request. Dropping the old receiver
/// implicitly cancels the in-flight request. Stale results are detected via
/// monotonically increasing request IDs.
pub struct AsyncPreviewLoader {
    generator: PreviewGenerator,
    receiver: Option<mpsc::Receiver<PreviewResult>>,
    current_id: u64,
    current_target: Option<String>,
    current_dimensions: Option<(u16, u16)>,
}

impl AsyncPreviewLoader {
    pub fn new(generator: PreviewGenerator) -> Self {
        Self {
            generator,
            receiver: None,
            current_id: 0,
            current_target: None,
            current_dimensions: None,
        }
    }

    pub fn request(&mut self, entry: &Entry, dimensions: Option<(u16, u16)>) {
        self.current_id = next_id();
        self.current_target = Some(entry.target.clone());
        self.current_dimensions = dimensions;

        let entry = entry.clone();
        let request_id = self.current_id;
        let (tx, rx) = mpsc::channel();
        self.receiver = Some(rx);

        let generator = self.generator.clone_for_thread();

        thread::spawn(move || {
            let (sender, receiver) = mpsc::channel();
            let target = entry.target.clone();
            let preview_dimensions = dimensions;

            thread::spawn(move || {
                let result = generator.generate(&entry, preview_dimensions);
                let _ = sender.send(result);
            });

            let preview_content = match receiver.recv_timeout(PREVIEW_TIMEOUT) {
                Ok(Ok(c)) => c,
                Ok(Err(e)) => PreviewState::Error(e),
                Err(_) => PreviewState::Error("preview timeout".to_string()),
            };

            let _ = tx.send(PreviewResult {
                id: request_id,
                target,
                dimensions: preview_dimensions,
                content: preview_content,
            });
        });
    }

    pub fn is_pending_for(&self, target: &str, dimensions: Option<(u16, u16)>) -> bool {
        self.receiver.is_some()
            && self.current_target.as_deref() == Some(target)
            && self.current_dimensions == dimensions
    }

    /// Non-blocking poll. Returns `None` when no result is ready or the
    /// result is stale (ID / target mismatch from a superseded request).
    pub fn poll(&mut self) -> Option<PreviewState> {
        let result = {
            let receiver = self.receiver.as_ref()?;
            match receiver.try_recv() {
                Ok(result) => result,
                Err(mpsc::TryRecvError::Empty) | Err(mpsc::TryRecvError::Disconnected) => {
                    return None;
                }
            }
        };
        self.receiver = None;

        if result.id != self.current_id {
            return None;
        }
        if self.current_target.as_deref() != Some(&result.target) {
            return None;
        }
        if self.current_dimensions != result.dimensions {
            return None;
        }

        Some(result.content)
    }

    pub fn clear(&mut self) {
        self.receiver = None;
        self.current_target = None;
        self.current_dimensions = None;
        self.current_id = 0;
    }
}

// =====================================================================
// Tests
// =====================================================================
#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapters::tmux::FakeTmuxSource;
    use crate::adapters::tmux::{RawSession, RawWindow, TmuxSource};
    use crate::domain::error::{ActionError, AdapterError};

    fn make_loader() -> AsyncPreviewLoader {
        let tmux = FakeTmuxSource {
            windows: vec![],
            sessions: vec![],
            current_session_name: "s".into(),
            current_window_idx: "0".into(),
            existing_sessions: vec![],
            fail_on: vec![],
        };
        let generator =
            PreviewGenerator::with_factory(Box::new(tmux), || Box::new(FakeTmuxSource::new()));
        AsyncPreviewLoader::new(generator)
    }

    fn make_window_entry() -> Entry {
        Entry::window("s".into(), "0".into(), "main".into(), "/tmp".into(), crate::domain::entry::SortPriority::CurrentWindow, true, None)
    }

    fn make_invalid_zoxide_entry() -> Entry {
        Entry::zoxide(
            "invalid".to_string(),
            "/definitely/missing/path/for/preview-loader".to_string(),
        )
    }

    struct SlowTmuxSource;

    impl TmuxSource for SlowTmuxSource {
        fn list_windows(&self) -> Result<Vec<RawWindow>, AdapterError> {
            Ok(vec![])
        }

        fn list_sessions(&self) -> Result<Vec<RawSession>, AdapterError> {
            Ok(vec![])
        }

        fn current_session(&self) -> Result<String, AdapterError> {
            Ok("s".to_string())
        }

        fn current_window_index(&self) -> Result<String, AdapterError> {
            Ok("0".to_string())
        }

        fn has_session(&self, _name: &str) -> Result<bool, AdapterError> {
            Ok(false)
        }

        fn select_window(&self, _target: &str) -> Result<(), ActionError> {
            Ok(())
        }

        fn new_session(&self, _name: &str, _path: &str) -> Result<(), ActionError> {
            Ok(())
        }

        fn new_window(&self, _session: &str, _path: &str) -> Result<String, ActionError> {
            Ok("s:1".to_string())
        }

        fn switch_client(&self, _target: &str) -> Result<(), ActionError> {
            Ok(())
        }

        fn kill_window(&self, _target: &str) -> Result<(), ActionError> {
            Ok(())
        }

        fn kill_session(&self, _name: &str) -> Result<(), ActionError> {
            Ok(())
        }

        fn capture_pane(&self, _target: &str, _line_count: usize) -> Result<String, AdapterError> {
            std::thread::sleep(PREVIEW_TIMEOUT + Duration::from_millis(150));
            Ok("late-pane".to_string())
        }

        fn capture_pane_with_size(
            &self,
            _target: &str,
            _line_count: usize,
            _width: Option<u16>,
            _height: Option<u16>,
        ) -> Result<String, AdapterError> {
            std::thread::sleep(PREVIEW_TIMEOUT + Duration::from_millis(150));
            Ok("late-pane".to_string())
        }
    }

    fn make_slow_loader() -> AsyncPreviewLoader {
        let generator =
            PreviewGenerator::with_factory(Box::new(SlowTmuxSource), || Box::new(SlowTmuxSource));
        AsyncPreviewLoader::new(generator)
    }

    #[test]
    fn new_loader_has_no_receiver() {
        let loader = make_loader();
        assert!(loader.receiver.is_none());
        assert!(loader.current_target.is_none());
    }

    #[test]
    fn request_creates_receiver_and_sets_target() {
        let mut loader = make_loader();
        let entry = make_window_entry();
        loader.request(&entry, None);

        assert!(loader.receiver.is_some());
        assert_eq!(loader.current_target.as_deref(), Some("s:0"));
        assert!(loader.current_id > 0);
    }

    #[test]
    fn clear_resets_state() {
        let mut loader = make_loader();
        loader.request(&make_window_entry(), None);
        loader.clear();

        assert!(loader.receiver.is_none());
        assert!(loader.current_target.is_none());
        assert_eq!(loader.current_id, 0);
    }

    #[test]
    fn request_twice_cancels_first() {
        let mut loader = make_loader();
        let entry1 = make_window_entry();
        let entry2 = Entry::window("s2".into(), "1".into(), "other".into(), "/tmp".into(), crate::domain::entry::SortPriority::OtherSessionWindow, false, None);

        loader.request(&entry1, None);
        let first_id = loader.current_id;
        let first_rx = loader.receiver.take().unwrap();

        loader.request(&entry2, None);
        assert_ne!(loader.current_id, first_id);
        drop(first_rx);
    }

    #[test]
    fn poll_returns_none_when_no_receiver() {
        let mut loader = make_loader();
        assert!(loader.poll().is_none());
    }

    #[test]
    fn poll_does_not_panic_without_result() {
        let mut loader = make_loader();
        loader.request(&make_window_entry(), None);
        let _ = loader.poll();
    }

    #[test]
    fn poll_returns_content_after_worker_finishes() {
        let mut loader = make_loader();
        loader.request(&make_window_entry(), None);
        thread::sleep(Duration::from_millis(200));

        let result = loader.poll();
        assert!(result.is_some());
        match result.unwrap() {
            PreviewState::DirectoryListing(content) => {
                assert_eq!(content.source, "tmux-fallback");
            }
            other => panic!("expected DirectoryListing fallback preview, got: {other:?}"),
        }
    }

    #[test]
    fn stale_result_discarded_on_id_mismatch() {
        let mut loader = make_loader();
        loader.request(&make_window_entry(), None);
        thread::sleep(Duration::from_millis(200));

        loader.current_id = next_id();
        assert!(loader.poll().is_none());
    }

    #[test]
    fn stale_result_discarded_on_target_mismatch() {
        let mut loader = make_loader();
        loader.request(&make_window_entry(), None);
        thread::sleep(Duration::from_millis(200));

        loader.current_target = Some("different:0".to_string());
        assert!(loader.poll().is_none());
    }

    #[test]
    fn stale_result_discarded_on_dimensions_mismatch() {
        let mut loader = make_loader();
        loader.request(&make_window_entry(), Some((80, 24)));
        thread::sleep(Duration::from_millis(200));

        loader.current_dimensions = Some((100, 24));
        assert!(loader.poll().is_none());
    }

    #[test]
    fn poll_consumes_receiver_after_result_arrives() {
        let mut loader = make_loader();
        loader.request(&make_window_entry(), None);
        thread::sleep(Duration::from_millis(200));

        let _ = loader.poll();
        assert!(loader.receiver.is_none());
    }

    #[test]
    fn poll_maps_generator_error_to_preview_error_state() {
        let mut loader = make_loader();
        loader.request(&make_invalid_zoxide_entry(), None);
        thread::sleep(Duration::from_millis(120));

        let result = loader
            .poll()
            .expect("loader should emit preview error state");
        match result {
            PreviewState::Error(message) => {
                assert!(message.contains("cannot read directory"));
            }
            other => panic!("expected PreviewState::Error, got: {other:?}"),
        }
    }

    #[test]
    fn poll_returns_timeout_error_when_generation_exceeds_timeout() {
        let mut loader = make_slow_loader();
        loader.request(&make_window_entry(), Some((80, 20)));

        thread::sleep(PREVIEW_TIMEOUT + Duration::from_millis(300));

        let result = loader
            .poll()
            .expect("loader should emit timeout error after deadline");
        assert_eq!(result, PreviewState::Error("preview timeout".to_string()));
    }
}
