use std::time::{Duration, Instant};

use ratatui::layout::{Constraint, Direction, Layout, Rect};

use crate::app::state::AppState;
use crate::preview::loader::AsyncPreviewLoader;
use crate::preview::types::PreviewState;
use crate::ui::theme::layout;

const PREVIEW_REFRESH_INTERVAL: Duration = Duration::from_millis(250);

#[derive(Debug, Default)]
pub(super) struct PreviewRequestTracker {
    pub(crate) last_preview_target: Option<String>,
    pub(crate) last_preview_dimensions: Option<(u16, u16)>,
    pub(crate) last_preview_requested_at: Option<Instant>,
}

impl PreviewRequestTracker {
    pub(super) fn reset(&mut self) {
        self.last_preview_target = None;
        self.last_preview_dimensions = None;
        self.last_preview_requested_at = None;
    }

    pub(super) fn trigger_if_needed(
        &mut self,
        state: &mut AppState,
        preview_loader: &mut AsyncPreviewLoader,
        dimensions: Option<(u16, u16)>,
    ) {
        if !state.preview_visible {
            return;
        }

        let entry = state.selected_entry();

        if let Some(entry) = entry {
            let target_changed = self.last_preview_target.as_deref() != Some(&entry.target);
            let dimensions_changed = self.last_preview_dimensions != dimensions;
            let pending_same_key = preview_loader.is_pending_for(&entry.target, dimensions);
            let interval_elapsed = self
                .last_preview_requested_at
                .is_none_or(|at| at.elapsed() >= PREVIEW_REFRESH_INTERVAL);

            if !target_changed && !dimensions_changed && (pending_same_key || !interval_elapsed) {
                return;
            }

            self.last_preview_target = Some(entry.target.clone());
            self.last_preview_dimensions = dimensions;
            self.last_preview_requested_at = Some(Instant::now());

            if target_changed || dimensions_changed {
                state.preview_state = PreviewState::Loading;
            }
            preview_loader.request(&entry, dimensions);
        } else {
            state.preview_state = PreviewState::Empty;
            preview_loader.clear();
            self.reset();
        }
    }

    pub(super) fn poll_into_state(
        &mut self,
        state: &mut AppState,
        preview_loader: &mut AsyncPreviewLoader,
    ) {
        if let Some(content) = preview_loader.poll() {
            state.preview_state = content;
        }
    }
}

pub(super) fn preview_dimensions(preview_visible: bool, terminal_area: Rect) -> Option<(u16, u16)> {
    if !preview_visible {
        return None;
    }

    let outer = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(1)])
        .split(terminal_area);

    let main_area = outer[0];
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(layout::LIST_WIDTH_PERCENT),
            Constraint::Percentage(layout::PREVIEW_WIDTH_PERCENT),
        ])
        .split(main_area);

    let preview_area = chunks[1];
    let width = preview_area.width.saturating_sub(2).max(1);
    let height = preview_area.height.saturating_sub(2).max(1);

    Some((width, height))
}
