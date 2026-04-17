use std::collections::HashMap;

use crate::adapters::fuzzy::{MatchResult, NucleoMatcher};
use crate::domain::entry::{Entry, EntryType};
use crate::domain::snapshot::Snapshot;

#[derive(Debug, Clone)]
pub enum GroupedListItem {
    SessionGroup {
        session: String,
        windows: Vec<Entry>,
    },
    StandaloneSession(Entry),
    ZoxideEntry(Entry),
}

#[derive(Debug, Clone)]
pub enum GroupedRow {
    SessionHeader {
        session: String,
        window_count: usize,
    },
    SessionWindow(Entry),
    StandaloneSession(Entry),
    ZoxideEntry(Entry),
}

impl GroupedRow {
    pub fn is_actionable(&self) -> bool {
        !matches!(self, Self::SessionHeader { .. })
    }

    pub fn actionable_entry(&self) -> Option<&Entry> {
        match self {
            GroupedRow::SessionHeader { .. } => None,
            GroupedRow::SessionWindow(entry)
            | GroupedRow::StandaloneSession(entry)
            | GroupedRow::ZoxideEntry(entry) => Some(entry),
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct GroupedList {
    pub items: Vec<GroupedListItem>,
}

impl GroupedList {
    pub fn from_snapshot(snapshot: &Snapshot) -> Self {
        let mut session_counts: HashMap<String, usize> = HashMap::new();
        for entry in &snapshot.entries {
            if entry.entry_type == EntryType::Window {
                if let Some(session) = &entry.session_name {
                    *session_counts.entry(session.clone()).or_insert(0) += 1;
                }
            }
        }

        let mut items: Vec<GroupedListItem> = Vec::new();
        let mut group_index_by_session: HashMap<String, usize> = HashMap::new();
        for entry in &snapshot.entries {
            match entry.entry_type {
                EntryType::Window => {
                    let Some(session) = entry.session_name.clone() else {
                        continue;
                    };
                    let count = session_counts.get(&session).copied().unwrap_or(0);

                    if count <= 1 {
                        items.push(GroupedListItem::StandaloneSession(entry.clone()));
                        continue;
                    }

                    if let Some(&idx) = group_index_by_session.get(&session) {
                        if let GroupedListItem::SessionGroup { windows, .. } = &mut items[idx] {
                            windows.push(entry.clone());
                        }
                    } else {
                        let new_idx = items.len();
                        group_index_by_session.insert(session.clone(), new_idx);
                        items.push(GroupedListItem::SessionGroup {
                            session,
                            windows: vec![entry.clone()],
                        });
                    }
                }
                EntryType::Zoxide => items.push(GroupedListItem::ZoxideEntry(entry.clone())),
            }
        }

        Self { items }
    }

    pub fn filtered_rows(&self, filter: &str, matcher: &NucleoMatcher) -> Vec<GroupedRow> {
        let trimmed_filter = filter.trim();
        let has_filter = !trimmed_filter.is_empty();

        if !has_filter {
            return self.all_rows();
        }

        let all_entries = self.actionable_entries();
        let matched = matcher.match_entries(trimmed_filter, &all_entries);

        self.build_filtered_rows(&matched)
    }

    fn build_filtered_rows(&self, matched: &[MatchResult]) -> Vec<GroupedRow> {
        let mut entry_data: std::collections::HashMap<String, (u32, Vec<u32>)> =
            std::collections::HashMap::with_capacity(matched.len());
        for result in matched {
            entry_data.insert(
                result.entry.target.clone(),
                (result.score, result.indices.clone()),
            );
        }

        let mut session_matches: Vec<(&String, Vec<Entry>)> = Vec::new();
        let mut standalone_matches: Vec<Entry> = Vec::new();
        let mut zoxide_matches: Vec<Entry> = Vec::new();

        for item in &self.items {
            match item {
                GroupedListItem::SessionGroup { session, windows } => {
                    let mut matched_windows: Vec<Entry> = windows
                        .iter()
                        .filter_map(|entry| {
                            entry_data.get(&entry.target).map(|(_, indices)| {
                                entry.clone().with_matched_indices(indices.clone())
                            })
                        })
                        .collect();

                    if !matched_windows.is_empty() {
                        matched_windows.sort_by_key(|e| {
                            std::cmp::Reverse(
                                entry_data.get(&e.target).map(|(s, _)| *s).unwrap_or(0),
                            )
                        });
                        session_matches.push((session, matched_windows));
                    }
                }
                GroupedListItem::StandaloneSession(entry) => {
                    if let Some((_, indices)) = entry_data.get(&entry.target) {
                        standalone_matches
                            .push(entry.clone().with_matched_indices(indices.clone()));
                    }
                }
                GroupedListItem::ZoxideEntry(entry) => {
                    if let Some((_, indices)) = entry_data.get(&entry.target) {
                        zoxide_matches.push(entry.clone().with_matched_indices(indices.clone()));
                    }
                }
            }
        }

        standalone_matches.sort_by_key(|e| {
            std::cmp::Reverse(entry_data.get(&e.target).map(|(s, _)| *s).unwrap_or(0))
        });
        zoxide_matches.sort_by_key(|e| {
            std::cmp::Reverse(entry_data.get(&e.target).map(|(s, _)| *s).unwrap_or(0))
        });

        let mut rows = Vec::new();
        for (session, windows) in session_matches {
            rows.push(GroupedRow::SessionHeader {
                session: session.clone(),
                window_count: windows.len(),
            });
            rows.extend(windows.into_iter().map(GroupedRow::SessionWindow));
        }
        for entry in standalone_matches {
            rows.push(GroupedRow::StandaloneSession(entry));
        }
        for entry in zoxide_matches {
            rows.push(GroupedRow::ZoxideEntry(entry));
        }

        rows
    }

    fn all_rows(&self) -> Vec<GroupedRow> {
        let mut rows = Vec::new();
        for item in &self.items {
            match item {
                GroupedListItem::SessionGroup { session, windows } => {
                    rows.push(GroupedRow::SessionHeader {
                        session: session.clone(),
                        window_count: windows.len(),
                    });
                    rows.extend(windows.iter().cloned().map(GroupedRow::SessionWindow));
                }
                GroupedListItem::StandaloneSession(entry) => {
                    rows.push(GroupedRow::StandaloneSession(entry.clone()));
                }
                GroupedListItem::ZoxideEntry(entry) => {
                    rows.push(GroupedRow::ZoxideEntry(entry.clone()));
                }
            }
        }
        rows
    }

    pub fn actionable_entries(&self) -> Vec<Entry> {
        let mut entries = Vec::new();
        for item in &self.items {
            match item {
                GroupedListItem::SessionGroup { windows, .. } => entries.extend(windows.clone()),
                GroupedListItem::StandaloneSession(entry) | GroupedListItem::ZoxideEntry(entry) => {
                    entries.push(entry.clone())
                }
            }
        }
        entries
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::entry::SortPriority;

    #[test]
    fn from_snapshot_groups_multi_window_sessions_in_input_order() {
        let snapshot = Snapshot::new(
            vec![
                Entry::window(
                    "s1".into(),
                    "0".into(),
                    "a".into(),
                    "/".into(),
                    SortPriority::CurrentWindow,
                    true,
                    None,
                    None,
                ),
                Entry::window(
                    "s1".into(),
                    "1".into(),
                    "b".into(),
                    "/".into(),
                    SortPriority::CurrentSessionOtherWindow,
                    false,
                    None,
                    None,
                ),
            ],
            "s1".into(),
            "s1:0".into(),
        );
        let grouped = GroupedList::from_snapshot(&snapshot);
        match &grouped.items[0] {
            GroupedListItem::SessionGroup { session, windows } => {
                assert_eq!(session, "s1");
                assert_eq!(windows.len(), 2);
                assert_eq!(windows[0].target, "s1:0");
                assert_eq!(windows[1].target, "s1:1");
            }
            other => panic!("expected SessionGroup, got {other:?}"),
        }
    }

    #[test]
    fn fuzzy_filter_matches_partial_chars() {
        let snapshot = Snapshot::new(
            vec![
                Entry::window(
                    "s1".into(),
                    "0".into(),
                    "main".into(),
                    "/".into(),
                    SortPriority::OtherSessionWindow,
                    false,
                    None,
                    None,
                ),
                Entry::window(
                    "s1".into(),
                    "1".into(),
                    "Makefile".into(),
                    "/".into(),
                    SortPriority::OtherSessionWindow,
                    false,
                    None,
                    None,
                ),
            ],
            "s1".into(),
            "s1:0".into(),
        );
        let grouped = GroupedList::from_snapshot(&snapshot);
        let matcher = NucleoMatcher::new();

        let rows = grouped.filtered_rows("mf", &matcher);
        let actionable_count = rows.iter().filter(|r| r.is_actionable()).count();
        assert_eq!(actionable_count, 1, "Should match only 'Makefile' window");
    }
}
