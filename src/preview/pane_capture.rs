use crate::adapters::tmux::TmuxSource;
use crate::domain::entry::Entry;
use crate::domain::error::AdapterError;

pub fn capture_pane_lines(
    tmux: &dyn TmuxSource,
    entry: &Entry,
    dimensions: Option<(u16, u16)>,
    max_pane_lines: usize,
) -> Result<Vec<String>, AdapterError> {
    let line_count = dimensions
        .map(|(_, height)| height as usize)
        .unwrap_or(max_pane_lines)
        .max(1)
        .min(max_pane_lines);
    let capture_width = dimensions.map(|(width, _)| width);
    let capture_height = dimensions.map(|(_, height)| height);

    let content =
        tmux.capture_pane_with_size(&entry.target, line_count, capture_width, capture_height)?;

    Ok(tail_lines_preserve_layout(&content, line_count))
}

pub fn tail_lines_preserve_layout(content: &str, max_lines: usize) -> Vec<String> {
    if max_lines == 0 {
        return vec![];
    }

    let normalized = content.strip_suffix('\n').unwrap_or(content);
    let all_lines: Vec<&str> = normalized.split('\n').collect();
    let start = all_lines.len().saturating_sub(max_lines);
    all_lines[start..]
        .iter()
        .map(|line| (*line).to_string())
        .collect()
}
