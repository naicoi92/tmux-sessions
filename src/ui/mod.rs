pub mod list;
pub mod preview;
pub mod theme;

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::Style,
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

use crate::app::state::AppState;
use crate::ui::theme::{colors, layout, styles};

pub fn render(frame: &mut Frame, state: &AppState) {
    let area = frame.area();
    let main_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(layout::HELP_HEIGHT)])
        .split(area);

    let content_area = main_layout[0];
    let help_area = main_layout[1];

    if state.preview_visible {
        let content_layout = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(layout::LIST_WIDTH_PERCENT),
                Constraint::Percentage(layout::PREVIEW_WIDTH_PERCENT),
            ])
            .split(content_area);

        let sidebar = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(layout::SEARCH_HEIGHT),
                Constraint::Min(1),
            ])
            .split(content_layout[0]);

        list::render_filter_bar(frame, sidebar[0], state);
        list::render_list(frame, sidebar[1], state);
        preview::render_preview(frame, content_layout[1], state);
    } else {
        let sidebar = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(layout::SEARCH_HEIGHT),
                Constraint::Min(1),
            ])
            .split(content_area);

        list::render_filter_bar(frame, sidebar[0], state);
        list::render_list(frame, sidebar[1], state);
    }
    render_help_bar(frame, help_area, state);
}

fn render_help_bar(frame: &mut Frame, area: Rect, state: &AppState) {
    let version = Span::styled(
        format!(" v{} ", env!("CARGO_PKG_VERSION")),
        styles::help_version(),
    );

    let status = if let Some(msg) = &state.status_message {
        Span::styled(
            format!(" {msg} "),
            Style::default().fg(colors::ACCENT_YELLOW),
        )
    } else {
        Span::raw("")
    };

    let help_items = [
        ("↑↓", "navigate"),
        ("Enter", "goto"),
        ("Ctrl+D", "kill"),
        ("Esc", "quit"),
    ];

    let help_spans: Vec<Span> = help_items
        .iter()
        .flat_map(|(key, desc)| {
            vec![
                Span::styled(format!(" {} ", key), styles::help_key()),
                Span::styled(*desc, styles::help_description()),
            ]
        })
        .collect();

    let mut spans = vec![version];
    spans.extend(help_spans);
    spans.push(status);

    let line = Line::from(spans);

    let bar = Paragraph::new(line);

    frame.render_widget(bar, area);
}
