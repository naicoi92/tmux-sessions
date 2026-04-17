use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, HighlightSpacing, List, ListItem, ListState, Paragraph},
    Frame,
};

use crate::app::state::AppState;
use crate::domain::entry::Entry;
use crate::domain::grouped_list::GroupedRow;
use crate::ui::theme::{colors, icons, layout, styles};

pub fn render_list(frame: &mut Frame, area: Rect, state: &AppState) {
    let filtered = state.filtered_rows();
    let selected = state.selected_visible_index();
    let item_count = filtered.len();
    let actionable_count = filtered.iter().filter(|row| row.is_actionable()).count();

    let items: Vec<ListItem> = filtered
        .iter()
        .enumerate()
        .map(|(idx, row)| {
            let (_style, spans) = style_for_row(row, idx);
            let mut line_spans = vec![Span::raw(" ")];
            line_spans.extend(spans);
            ListItem::new(Line::from(line_spans))
        })
        .collect();

    let block = Block::default()
        .title(format!(
            " {} Sessions ({}/{}) ",
            icons::DIRECTORY,
            actionable_count,
            item_count
        ))
        .title_style(styles::list_header())
        .borders(Borders::ALL)
        .border_set(ratatui::symbols::border::ROUNDED)
        .border_style(styles::block_default());

    let list = List::new(items)
        .block(block)
        .highlight_style(styles::list_item_selected())
        .highlight_symbol(layout::HIGHLIGHT_SYMBOL)
        .highlight_spacing(HighlightSpacing::Always);

    let mut list_state = ListState::default();
    if filtered.is_empty() {
        list_state.select(None);
    } else {
        list_state.select(selected);
    }

    frame.render_stateful_widget(list, area, &mut list_state);
}

fn render_entry_with_highlight<'a>(
    entry: &'a Entry,
    base_style: Style,
    highlight_style: Style,
    prefix: &'a str,
) -> Vec<Span<'a>> {
    let display = entry.display.trim_end();

    let marker_char_count = count_leading_markers(display);
    let stripped_display: String = display.chars().skip(marker_char_count).collect();

    let prefix_char_count = prefix.chars().count() + 1;

    let full_text = format!("{} {}", prefix, stripped_display);

    if entry.matched_indices.is_empty() {
        return vec![Span::styled(full_text, base_style)];
    }

    let chars: Vec<char> = full_text.chars().collect();

    let mut adjusted_indices: Vec<u32> = entry
        .matched_indices
        .iter()
        .filter_map(|&i| {
            let marker_chars = marker_char_count as u32;
            if i >= marker_chars {
                let adjusted = i - marker_chars + prefix_char_count as u32;
                if (adjusted as usize) < chars.len() {
                    Some(adjusted)
                } else {
                    None
                }
            } else {
                None
            }
        })
        .collect();
    adjusted_indices.sort_unstable();

    let mut spans = Vec::new();
    let mut current_span = String::new();
    let mut in_highlight = false;
    let mut indices_idx = 0;

    for (i, c) in chars.iter().enumerate() {
        let i_u32 = i as u32;
        let should_highlight =
            indices_idx < adjusted_indices.len() && adjusted_indices[indices_idx] == i_u32;

        if should_highlight && indices_idx < adjusted_indices.len() {
            indices_idx += 1;
        }

        if should_highlight != in_highlight {
            if !current_span.is_empty() {
                let style = if in_highlight {
                    highlight_style
                } else {
                    base_style
                };
                spans.push(Span::styled(current_span.clone(), style));
                current_span.clear();
            }
            in_highlight = should_highlight;
        }
        current_span.push(*c);
    }

    if !current_span.is_empty() {
        let style = if in_highlight {
            highlight_style
        } else {
            base_style
        };
        spans.push(Span::styled(current_span, style));
    }

    spans
}

fn count_leading_markers(display: &str) -> usize {
    display
        .chars()
        .enumerate()
        .take_while(|(i, c)| {
            if *i % 2 == 0 {
                *c == ' ' || *c == '▸' || *c == '◆' || *c == '▤'
            } else {
                *c == ' '
            }
        })
        .count()
}

fn style_for_row(row: &GroupedRow, _index: usize) -> (Style, Vec<Span<'_>>) {
    match row {
        GroupedRow::SessionHeader {
            session,
            window_count,
        } => {
            let text = format!("{} {} ({})", icons::SESSION_EXPANDED, session, window_count);
            (
                styles::list_header(),
                vec![Span::styled(text, styles::list_header())],
            )
        }
        GroupedRow::SessionWindow(entry) => {
            let base_style = if entry.is_current {
                styles::text_current()
            } else {
                styles::list_item_normal()
            };
            let highlight_style = base_style
                .add_modifier(Modifier::BOLD)
                .fg(colors::ACCENT_YELLOW);
            let spans =
                render_entry_with_highlight(entry, base_style, highlight_style, icons::ARROW_SUB);
            (base_style, spans)
        }
        GroupedRow::StandaloneSession(entry) => {
            let base_style = if entry.is_current {
                styles::text_current()
            } else {
                styles::list_item_normal()
            };
            let highlight_style = base_style
                .add_modifier(Modifier::BOLD)
                .fg(colors::ACCENT_YELLOW);
            let spans =
                render_entry_with_highlight(entry, base_style, highlight_style, icons::WINDOW);
            (base_style, spans)
        }
        GroupedRow::ZoxideEntry(entry) => {
            let base_style = Style::default().fg(colors::ACCENT_BLUE);
            let highlight_style = base_style
                .add_modifier(Modifier::BOLD)
                .fg(colors::ACCENT_CYAN);
            let spans =
                render_entry_with_highlight(entry, base_style, highlight_style, icons::FOLDER);
            (base_style, spans)
        }
    }
}

pub fn render_filter_bar(frame: &mut Frame, area: Rect, state: &AppState) {
    let has_filter = !state.filter.is_empty();

    let block = Block::default()
        .borders(Borders::ALL)
        .border_set(ratatui::symbols::border::ROUNDED)
        .border_style(if has_filter {
            styles::block_focused()
        } else {
            styles::block_default()
        });

    let search_icon = Span::styled(format!("{} ", icons::SEARCH), styles::help_key());

    let line = if has_filter {
        let before_cursor = &state.filter[..state.filter_cursor];
        let after_cursor = &state.filter[state.filter_cursor..];

        Line::from(vec![
            search_icon,
            Span::styled(before_cursor.to_string(), styles::search_text()),
            Span::styled(
                icons::CURSOR,
                styles::text_highlighted().fg(colors::ACCENT_CYAN),
            ),
            Span::styled(after_cursor.to_string(), styles::search_text()),
        ])
    } else {
        Line::from(vec![
            search_icon,
            Span::styled(" type to search...", styles::search_placeholder()),
        ])
    };

    let text_widget = Paragraph::new(line).block(block);
    frame.render_widget(text_widget, area);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::entry::{Entry, SortPriority};

    #[test]
    fn highlight_indices_adjusted_correctly() {
        let mut entry = Entry::window("test".into(), "0".into(), "alpha".into(), "/path".into(), SortPriority::OtherSessionWindow, false, None);
        entry.matched_indices = vec![10, 11, 12, 13, 14];

        let base_style = Style::default();
        let highlight_style = Style::default().add_modifier(Modifier::BOLD);

        let spans = render_entry_with_highlight(&entry, base_style, highlight_style, "◆");

        assert!(!spans.is_empty());
    }
}
