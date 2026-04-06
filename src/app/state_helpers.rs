use crate::domain::entry::Entry;
use crate::domain::grouped_list::GroupedRow;

pub(super) fn actionable_count(rows: &[GroupedRow]) -> usize {
    rows.iter().filter(|row| row.is_actionable()).count()
}

pub(super) fn selected_actionable_entry(
    rows: &[GroupedRow],
    selected_index: usize,
) -> Option<Entry> {
    let mut actionable_idx = 0usize;
    for row in rows {
        if let Some(entry) = row.actionable_entry() {
            if actionable_idx == selected_index {
                return Some(entry.clone());
            }
            actionable_idx += 1;
        }
    }
    None
}

pub(super) fn selected_visible_index(rows: &[GroupedRow], selected_index: usize) -> Option<usize> {
    let mut actionable_idx = 0usize;
    for (row_idx, row) in rows.iter().enumerate() {
        if row.is_actionable() {
            if actionable_idx == selected_index {
                return Some(row_idx);
            }
            actionable_idx += 1;
        }
    }
    None
}

pub(super) fn restore_selection(
    rows: &[GroupedRow],
    selected_index: usize,
    preferred_target: Option<String>,
    anchor_visible_index: Option<usize>,
) -> (usize, Option<String>) {
    let actionable_rows: Vec<(usize, Entry)> = rows
        .iter()
        .enumerate()
        .filter_map(|(row_idx, row)| {
            row.actionable_entry()
                .cloned()
                .map(|entry| (row_idx, entry))
        })
        .collect();

    if actionable_rows.is_empty() {
        return (0, None);
    }

    if let Some(target) = preferred_target {
        if let Some((idx, (_, entry))) = actionable_rows
            .iter()
            .enumerate()
            .find(|(_, (_, entry))| entry.target == target)
        {
            return (idx, Some(entry.target.clone()));
        }
    }

    if let Some(anchor) = anchor_visible_index {
        let mut best_idx = 0usize;
        let mut best_dist = usize::MAX;
        let mut best_prefers_after = false;

        for (idx, (row_idx, _)) in actionable_rows.iter().enumerate() {
            let dist = row_idx.abs_diff(anchor);
            let prefers_after = *row_idx >= anchor;

            if dist < best_dist || (dist == best_dist && prefers_after && !best_prefers_after) {
                best_dist = dist;
                best_idx = idx;
                best_prefers_after = prefers_after;
            }
        }

        return (best_idx, Some(actionable_rows[best_idx].1.target.clone()));
    }

    let clamped = selected_index.min(actionable_rows.len().saturating_sub(1));
    (clamped, Some(actionable_rows[clamped].1.target.clone()))
}

pub(super) fn insert_filter_char(filter: &mut String, filter_cursor: &mut usize, ch: char) {
    filter.insert(*filter_cursor, ch);
    *filter_cursor += 1;
}

pub(super) fn delete_filter_char(filter: &mut String, filter_cursor: &mut usize) {
    if *filter_cursor > 0 && !filter.is_empty() {
        filter.remove(*filter_cursor - 1);
        *filter_cursor -= 1;
    }
}

pub(super) fn clear_filter_with_cursor(filter: &mut String, filter_cursor: &mut usize) {
    filter.clear();
    *filter_cursor = 0;
}
