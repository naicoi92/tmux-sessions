use ansi_to_tui::IntoText;
use ratatui::{
    layout::{Alignment, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};

use crate::app::state::AppState;
use crate::domain::entry::EntryType;
use crate::preview::types::{DirectoryListingContent, PreviewState, TmuxScreenContent};
use crate::ui::theme::{colors, icons, styles};

fn scroll_to_bottom_offset(content_lines: usize, visible_height: usize) -> u16 {
    content_lines.saturating_sub(visible_height) as u16
}

pub fn render_preview(frame: &mut Frame, area: Rect, state: &AppState) {
    let title = state
        .selected_entry()
        .map(|e| {
            let icon = match e.entry_type {
                EntryType::Window => icons::TERMINAL,
                EntryType::Zoxide => icons::DIRECTORY,
            };
            format!(" {icon} {} ", e.path)
        })
        .unwrap_or_else(|| format!(" {} Preview ", icons::PREVIEW));

    let block = Block::default()
        .title(title)
        .title_style(styles::preview_header())
        .borders(Borders::ALL)
        .border_set(ratatui::symbols::border::ROUNDED)
        .border_style(styles::block_default());

    let inner = block.inner(area);
    let visible_width = inner.width.max(1) as usize;
    let visible_height = inner.height.max(1) as usize;

    if matches!(state.preview_state, PreviewState::Loading) {
        let loading_text = format!(" {} Loading...", icons::LOADING);
        let loading_widget = Paragraph::new(loading_text)
            .style(styles::text_current())
            .alignment(Alignment::Center);

        let centered_layout = ratatui::layout::Layout::default()
            .direction(ratatui::layout::Direction::Vertical)
            .constraints([
                ratatui::layout::Constraint::Percentage(50),
                ratatui::layout::Constraint::Length(1),
                ratatui::layout::Constraint::Percentage(50),
            ])
            .split(inner);

        frame.render_widget(loading_widget, centered_layout[1]);
        frame.render_widget(block, area);
        return;
    }

    let (content, scroll_offset, wrap_trim) = match &state.preview_state {
        PreviewState::Empty => {
            let placeholder = Paragraph::new("No selection").style(styles::text_muted());
            frame.render_widget(placeholder.block(block), area);
            return;
        }
        state => render_state(state, visible_width, visible_height),
    };

    let mut paragraph = Paragraph::new(content).scroll((scroll_offset, 0));
    if let Some(trim) = wrap_trim {
        paragraph = paragraph.wrap(Wrap { trim });
    }
    frame.render_widget(paragraph.block(block), area);
}

fn render_state(
    state: &PreviewState,
    visible_width: usize,
    visible_height: usize,
) -> (Vec<Line<'static>>, u16, Option<bool>) {
    match state {
        PreviewState::TmuxScreen(content) => {
            render_tmux_screen(content, visible_width, visible_height)
        }
        PreviewState::DirectoryListing(content) => render_directory_listing(content, visible_width),
        PreviewState::Loading => {
            let lines = vec![Line::from(Span::styled(
                format!(" {} Loading...", icons::LOADING),
                styles::text_current(),
            ))];
            (lines, 0, Some(true))
        }
        PreviewState::Error(msg) => {
            let lines = vec![
                Line::from(Span::styled(
                    format!("{} Preview error", icons::CROSS),
                    Style::default()
                        .fg(colors::ACCENT_RED)
                        .add_modifier(Modifier::BOLD),
                )),
                Line::from(Span::styled(format!(" {msg}"), styles::text_muted())),
            ];
            (lines, 0, Some(true))
        }
        PreviewState::Empty => (vec![], 0, Some(true)),
        _ => {
            let lines = vec![Line::from(Span::styled(
                " Legacy preview variant is no longer rendered",
                styles::text_muted(),
            ))];
            (lines, 0, Some(true))
        }
    }
}

fn render_tmux_screen(
    content: &TmuxScreenContent,
    visible_width: usize,
    visible_height: usize,
) -> (Vec<Line<'static>>, u16, Option<bool>) {
    let lines = if content.screen_lines.is_empty() {
        vec![Line::from(Span::styled("(empty)", styles::text_muted()))]
    } else {
        render_ansi_pane_lines(&content.screen_lines, visible_width)
    };

    let scroll_offset = scroll_to_bottom_offset(lines.len(), visible_height);
    (lines, scroll_offset, None)
}

fn render_directory_listing(
    content: &DirectoryListingContent,
    visible_width: usize,
) -> (Vec<Line<'static>>, u16, Option<bool>) {
    let mut lines = Vec::new();

    if content.entries.is_empty() {
        lines.push(Line::from(Span::styled(" (empty)", styles::text_muted())));
    } else {
        for entry in &content.entries {
            lines.extend(wrap_ansi_line(entry, visible_width));
        }
    }

    (lines, 0, Some(true))
}

fn wrap_ansi_line(line: &str, width: usize) -> Vec<Line<'static>> {
    use ansi_to_tui::IntoText;

    let text = match line.into_text() {
        Ok(t) => t,
        Err(_) => {
            return vec![Line::from(Span::raw(line.to_string()))];
        }
    };

    let mut result = Vec::new();
    for text_line in text.lines {
        let line_str = text_line.to_string();
        if line_str.len() > width {
            result.push(Line::from(Span::raw(
                line_str
                    .chars()
                    .take(width.saturating_sub(1))
                    .collect::<String>()
                    + "…",
            )));
        } else {
            result.push(text_line);
        }
    }

    if result.is_empty() {
        result.push(Line::from(Span::raw(line.to_string())));
    }

    result
}

fn render_ansi_pane_lines(pane: &[String], _visible_width: usize) -> Vec<Line<'static>> {
    let raw = pane.join("\n");
    match raw.into_text() {
        Ok(text) => {
            let owned = text.to_owned();
            if owned.lines.is_empty() {
                vec![Line::from(Span::styled(" (empty)", styles::text_muted()))]
            } else {
                owned.lines
            }
        }
        Err(_) => pane
            .iter()
            .map(|line| Line::from(Span::raw(line.clone())))
            .collect(),
    }
}

#[allow(dead_code)]
fn render_contextual(
    _title: &str,
    _name: &str,
    _path: &str,
    _headline: &str,
    _details: &[String],
    _pane_content: Option<&[String]>,
    _visible_width: usize,
) -> Vec<Line<'static>> {
    Vec::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scroll_offset_shows_bottom_lines() {
        assert_eq!(scroll_to_bottom_offset(20, 5), 15);
    }
}
