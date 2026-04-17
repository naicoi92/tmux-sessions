use crossterm::event::{self, Event};
use ratatui::{layout::Rect, Terminal};

use crate::adapters::tmux::{FakeTmuxSource, TmuxAdapter, TmuxSource};
use crate::app::event_action_coordinator::{handle_action, LoopOutcome};
use crate::app::events::map_key_to_action;
use crate::app::loader::SnapshotLoader;
use crate::app::preview_orchestrator::{preview_dimensions, PreviewRequestTracker};
use crate::app::state::AppState;
pub use crate::app::terminal_lifecycle::{init_terminal, restore_terminal};
use crate::domain::snapshot::Snapshot;
use crate::preview::generator::PreviewGenerator;
use crate::preview::loader::AsyncPreviewLoader;
use crate::ui;

type PreviewTmuxFactory = fn() -> Box<dyn TmuxSource + Send>;

pub struct AppController {
    state: AppState,
    loader: SnapshotLoader,
    tmux: Box<dyn TmuxSource>,
    preview_loader: AsyncPreviewLoader,
    preview_tracker: PreviewRequestTracker,
}

impl AppController {
    pub fn new<F>(
        loader: SnapshotLoader,
        tmux: Box<dyn TmuxSource>,
        preview_tmux_factory: F,
        snapshot: Snapshot,
    ) -> Self
    where
        F: Fn() -> Box<dyn TmuxSource + Send> + Send + Sync + 'static,
    {
        let preview_tmux = preview_tmux_factory();
        let generator = PreviewGenerator::with_factory(preview_tmux, preview_tmux_factory);
        let preview_loader = AsyncPreviewLoader::new(generator);
        Self {
            state: AppState::new(snapshot),
            loader,
            tmux,
            preview_loader,
            preview_tracker: PreviewRequestTracker::default(),
        }
    }

    fn reload_snapshot(&mut self) {
        if let Ok(snap) = self.loader.load() {
            self.state.replace_snapshot(snap);
            self.state.clear_filter();
            self.preview_tracker.reset();
        }
    }

    fn trigger_preview_if_needed(&mut self, dimensions: Option<(u16, u16)>) {
        self.preview_tracker.trigger_if_needed(
            &mut self.state,
            &mut self.preview_loader,
            dimensions,
        );
    }

    fn poll_preview(&mut self) {
        self.preview_tracker
            .poll_into_state(&mut self.state, &mut self.preview_loader);
    }

    pub fn run<B: ratatui::backend::Backend>(
        &mut self,
        terminal: &mut Terminal<B>,
    ) -> Result<ExitAction, Box<dyn std::error::Error>>
    where
        <B as ratatui::backend::Backend>::Error: 'static,
    {
        loop {
            if let Ok(area) = terminal.size() {
                let terminal_area = Rect::new(0, 0, area.width, area.height);
                let dimensions = preview_dimensions(self.state.preview_visible, terminal_area);
                self.trigger_preview_if_needed(dimensions);
            }

            self.poll_preview();
            terminal.draw(|frame| ui::render(frame, &self.state))?;

            if !event::poll(std::time::Duration::from_millis(100))? {
                continue;
            }

            let event = event::read()?;
            match event {
                Event::Key(key) => {
                    let action = map_key_to_action(key);
                    match handle_action(action, &mut self.state, self.tmux.as_ref())? {
                        LoopOutcome::Continue => {}
                        LoopOutcome::Quit => return Ok(ExitAction::Quit),
                        LoopOutcome::SwitchTo(target) => return Ok(ExitAction::SwitchTo(target)),
                        LoopOutcome::ReloadSnapshot => self.reload_snapshot(),
                    }
                }
                Event::Resize(_, _) => {}
                _ => {}
            }

            if self.state.should_quit {
                return Ok(ExitAction::Quit);
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExitAction {
    Quit,
    SwitchTo(String),
}

pub fn run_app(
    loader: SnapshotLoader,
    debug: bool,
) -> Result<ExitAction, Box<dyn std::error::Error>> {
    fn real_tmux_source() -> Box<dyn TmuxSource + Send> {
        Box::new(TmuxAdapter::new())
    }

    fn fake_tmux_source() -> Box<dyn TmuxSource + Send> {
        Box::new(FakeTmuxSource::new())
    }

    let snapshot = match loader.load() {
        Ok(snap) => snap,
        Err(e) => {
            eprintln!("warning: snapshot load failed: {e}, using empty snapshot");
            Snapshot::empty()
        }
    };

    let (tmux, preview_tmux_factory): (Box<dyn TmuxSource>, PreviewTmuxFactory) = if debug {
        (Box::new(FakeTmuxSource::new()), fake_tmux_source)
    } else {
        (Box::new(TmuxAdapter::new()), real_tmux_source)
    };

    let mut controller = AppController::new(loader, tmux, preview_tmux_factory, snapshot);
    let mut terminal = init_terminal()?;

    let result = controller.run(&mut terminal);

    restore_terminal()?;
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapters::tmux::{FakeTmuxSource, RawWindow};
    use crate::adapters::zoxide::FakeZoxideSource;
    use crate::app::loader::create_test_loader;
    use crate::domain::entry::Entry;
    use crate::domain::snapshot::Snapshot;
    use crate::preview::types::PreviewState;
    use std::thread;
    use std::time::{Duration, Instant};

    fn make_controller() -> AppController {
        let tmux = FakeTmuxSource {
            windows: vec![RawWindow {
                session_name: "s".into(),
                window_index: "0".into(),
                window_name: "main".into(),
                window_path: "/".into(),
                window_activity: None,
            }],
            sessions: vec![],
            current_session_name: "s".into(),
            current_window_idx: "0".into(),
            existing_sessions: vec!["s".into()],
            fail_on: vec![],
        };
        let zoxide = FakeZoxideSource { paths: vec![] };
        let loader = create_test_loader(Box::new(tmux), Box::new(zoxide));
        let snap = loader.load().unwrap();
        let action_tmux: Box<dyn TmuxSource> = Box::new(FakeTmuxSource::new());
        AppController::new(
            loader,
            action_tmux,
            || Box::new(FakeTmuxSource::new()),
            snap,
        )
    }

    fn make_loader_for_tests() -> SnapshotLoader {
        let tmux = FakeTmuxSource {
            windows: vec![RawWindow {
                session_name: "s".into(),
                window_index: "0".into(),
                window_name: "main".into(),
                window_path: "/".into(),
                window_activity: None,
            }],
            sessions: vec![],
            current_session_name: "s".into(),
            current_window_idx: "0".into(),
            existing_sessions: vec!["s".into()],
            fail_on: vec![],
        };
        let zoxide = FakeZoxideSource { paths: vec![] };
        create_test_loader(Box::new(tmux), Box::new(zoxide))
    }

    fn make_snapshot_with_single_zoxide(path: String) -> Snapshot {
        Snapshot::new(
            vec![Entry::zoxide("project".into(), path)],
            "s".into(),
            "s:0".into(),
        )
    }

    #[test]
    fn controller_creates_with_snapshot() {
        let ctrl = make_controller();
        assert_eq!(ctrl.state.snapshot.len(), 1);
        assert_eq!(ctrl.state.selected_index, 0);
    }

    #[test]
    fn exit_action_quit() {
        assert_eq!(ExitAction::Quit, ExitAction::Quit);
    }

    #[test]
    fn exit_action_switch_to() {
        let action = ExitAction::SwitchTo("s:0".into());
        assert_eq!(action, ExitAction::SwitchTo("s:0".into()));
    }

    #[test]
    fn preview_uses_injected_tmux_source_semantics_for_debug_like_mode() {
        let session_like_name = "tmux-popup-preview-debug-source";
        let path = format!("/tmp/{session_like_name}");
        let _ = std::fs::remove_dir_all(&path);
        std::fs::create_dir_all(&path).unwrap();
        std::fs::write(format!("{}/test.txt", path), "test content").unwrap();
        let mut action_tmux = FakeTmuxSource::new();
        action_tmux.existing_sessions = vec![session_like_name.to_string()];

        let loader = make_loader_for_tests();
        let snapshot = make_snapshot_with_single_zoxide(path.clone());
        let mut ctrl = AppController::new(
            loader,
            Box::new(action_tmux),
            move || {
                let mut preview_tmux = FakeTmuxSource::new();
                preview_tmux.existing_sessions = vec![session_like_name.to_string()];
                Box::new(preview_tmux)
            },
            snapshot,
        );

        ctrl.trigger_preview_if_needed(None);
        thread::sleep(Duration::from_millis(250));
        ctrl.poll_preview();

        match &ctrl.state.preview_state {
            PreviewState::DirectoryListing(content) => {
                assert!(
                    !content.entries.is_empty(),
                    "directory listing should have entries"
                );
            }
            other => panic!("expected DirectoryListing preview, got {other:?}"),
        }

        let _ = std::fs::remove_dir_all(&path);
    }

    #[test]
    fn preview_refreshes_even_when_selection_unchanged_after_interval() {
        let loader = make_loader_for_tests();
        let snapshot = Snapshot::new(
            vec![Entry::window(
                "s".into(),
                "0".into(),
                "main".into(),
                "/".into(),
                crate::domain::entry::SortPriority::CurrentWindow,
                true,
            )],
            "s".into(),
            "s:0".into(),
        );
        let mut ctrl = AppController::new(
            loader,
            Box::new(FakeTmuxSource::new()),
            || Box::new(FakeTmuxSource::new()),
            snapshot,
        );

        ctrl.trigger_preview_if_needed(None);
        thread::sleep(Duration::from_millis(250));
        ctrl.poll_preview();
        assert!(matches!(
            ctrl.state.preview_state,
            PreviewState::DirectoryListing(_)
        ));
        let first_requested_at = ctrl
            .preview_tracker
            .last_preview_requested_at
            .expect("first preview request timestamp should be set");

        thread::sleep(Duration::from_millis(300));
        ctrl.trigger_preview_if_needed(None);
        let second_requested_at = ctrl
            .preview_tracker
            .last_preview_requested_at
            .expect("second preview request timestamp should be set");

        assert!(second_requested_at > first_requested_at);
        assert!(matches!(
            ctrl.state.preview_state,
            PreviewState::DirectoryListing(_)
        ));
    }

    #[test]
    fn preview_refreshes_when_dimensions_change_without_waiting_interval() {
        let loader = make_loader_for_tests();
        let snapshot = Snapshot::new(
            vec![Entry::window(
                "s".into(),
                "0".into(),
                "main".into(),
                "/".into(),
                crate::domain::entry::SortPriority::CurrentWindow,
                true,
            )],
            "s".into(),
            "s:0".into(),
        );
        let mut ctrl = AppController::new(
            loader,
            Box::new(FakeTmuxSource::new()),
            || Box::new(FakeTmuxSource::new()),
            snapshot,
        );

        ctrl.trigger_preview_if_needed(Some((80, 20)));
        let first_requested_at = ctrl
            .preview_tracker
            .last_preview_requested_at
            .expect("first preview request timestamp should be set");

        ctrl.trigger_preview_if_needed(Some((100, 20)));
        let second_requested_at = ctrl
            .preview_tracker
            .last_preview_requested_at
            .expect("second preview request timestamp should be set");

        assert!(second_requested_at > first_requested_at);
    }

    #[test]
    fn preview_does_not_rerequest_same_key_when_previous_result_not_polled() {
        let loader = make_loader_for_tests();
        let snapshot = Snapshot::new(
            vec![Entry::window(
                "s".into(),
                "0".into(),
                "main".into(),
                "/".into(),
                crate::domain::entry::SortPriority::CurrentWindow,
                true,
            )],
            "s".into(),
            "s:0".into(),
        );
        let mut ctrl = AppController::new(
            loader,
            Box::new(FakeTmuxSource::new()),
            || Box::new(FakeTmuxSource::new()),
            snapshot,
        );

        ctrl.trigger_preview_if_needed(None);
        let first_requested_at = ctrl
            .preview_tracker
            .last_preview_requested_at
            .expect("first preview request timestamp should be set");

        thread::sleep(Duration::from_millis(300));
        ctrl.trigger_preview_if_needed(None);

        let second_requested_at = ctrl
            .preview_tracker
            .last_preview_requested_at
            .expect("second preview request timestamp should be set");
        assert_eq!(second_requested_at, first_requested_at);
    }

    #[test]
    fn preview_throttles_same_target_before_interval_after_poll() {
        let loader = make_loader_for_tests();
        let snapshot = Snapshot::new(
            vec![Entry::window(
                "s".into(),
                "0".into(),
                "main".into(),
                "/".into(),
                crate::domain::entry::SortPriority::CurrentWindow,
                true,
            )],
            "s".into(),
            "s:0".into(),
        );
        let mut ctrl = AppController::new(
            loader,
            Box::new(FakeTmuxSource::new()),
            || Box::new(FakeTmuxSource::new()),
            snapshot,
        );

        ctrl.trigger_preview_if_needed(None);
        thread::sleep(Duration::from_millis(250));
        ctrl.poll_preview();

        ctrl.preview_tracker.last_preview_requested_at = Some(Instant::now());

        let first_requested_at = ctrl
            .preview_tracker
            .last_preview_requested_at
            .expect("first preview request timestamp should be set");

        ctrl.trigger_preview_if_needed(None);

        let second_requested_at = ctrl
            .preview_tracker
            .last_preview_requested_at
            .expect("second preview request timestamp should be set");
        assert_eq!(second_requested_at, first_requested_at);
    }

    #[test]
    fn preview_not_retriggered_twice_in_same_tick_without_changes() {
        let loader = make_loader_for_tests();
        let snapshot = Snapshot::new(
            vec![Entry::window(
                "s".into(),
                "0".into(),
                "main".into(),
                "/".into(),
                crate::domain::entry::SortPriority::CurrentWindow,
                true,
            )],
            "s".into(),
            "s:0".into(),
        );
        let mut ctrl = AppController::new(
            loader,
            Box::new(FakeTmuxSource::new()),
            || Box::new(FakeTmuxSource::new()),
            snapshot,
        );

        // First call sets the timestamp
        ctrl.trigger_preview_if_needed(Some((80, 20)));
        let first_requested_at = ctrl
            .preview_tracker
            .last_preview_requested_at
            .expect("first preview request timestamp should be set");

        // Second call immediately after with same dimensions should NOT update timestamp
        ctrl.trigger_preview_if_needed(Some((80, 20)));
        let second_requested_at = ctrl
            .preview_tracker
            .last_preview_requested_at
            .expect("second preview request timestamp should be set");

        // Timestamps should be equal - second call was throttled
        assert_eq!(second_requested_at, first_requested_at);
    }

    #[test]
    fn preview_not_requested_while_panel_hidden() {
        let loader = make_loader_for_tests();
        let snapshot = Snapshot::new(
            vec![Entry::window(
                "s".into(),
                "0".into(),
                "main".into(),
                "/".into(),
                crate::domain::entry::SortPriority::CurrentWindow,
                true,
            )],
            "s".into(),
            "s:0".into(),
        );
        let mut ctrl = AppController::new(
            loader,
            Box::new(FakeTmuxSource::new()),
            || Box::new(FakeTmuxSource::new()),
            snapshot,
        );
        ctrl.state.toggle_preview();

        ctrl.trigger_preview_if_needed(None);

        assert!(ctrl.preview_tracker.last_preview_requested_at.is_none());
        assert!(matches!(ctrl.state.preview_state, PreviewState::Empty));
    }
}
