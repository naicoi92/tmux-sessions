use std::collections::HashMap;

use crate::adapters::fuzzy::{MatchResult, NucleoMatcher};
use crate::domain::entry::{Entry, EntryType};
use crate::domain::snapshot::Snapshot;

#[derive(Debug, Clone)]
pub enum GroupedListItem {
    SessionGroup {
        session: String,
        display_name: String,
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
        let mut path_by_session: HashMap<String, String> = HashMap::new();
        for entry in &snapshot.entries {
            if entry.entry_type == EntryType::Window {
                if let Some(session) = &entry.session_name {
                    *session_counts.entry(session.clone()).or_insert(0) += 1;
                    path_by_session
                        .entry(session.clone())
                        .or_insert_with(|| entry.path.clone());
                }
            }
        }

        // Disambiguate display names across unique sessions, not windows.
        // Multiple windows in one session must not force parent/path display.
        let all_session_paths: Vec<&str> = path_by_session.values().map(String::as_str).collect();
        let mut display_by_session: HashMap<String, String> = HashMap::new();
        for (session, path) in &path_by_session {
            let display = crate::domain::path_name::disambiguate_name(path, &all_session_paths);
            display_by_session.insert(session.clone(), display);
        }

        // When two sessions share the same path, display names collide.
        // Append tmux session name to distinguish: "project (work-session)".
        let mut display_counts: HashMap<String, Vec<String>> = HashMap::new();
        for (session, display) in &display_by_session {
            display_counts
                .entry(display.clone())
                .or_default()
                .push(session.clone());
        }
        for (display, sessions) in &display_counts {
            if sessions.len() > 1 {
                for session in sessions {
                    display_by_session.insert(session.clone(), format!("{display} ({session})"));
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

                    let display_name = display_by_session
                        .get(&session)
                        .cloned()
                        .unwrap_or_else(|| session.clone());
                    let entry = entry.clone().with_display_session_name(&display_name);

                    if count <= 1 {
                        items.push(GroupedListItem::StandaloneSession(entry));
                        continue;
                    }

                    if let Some(&idx) = group_index_by_session.get(&session) {
                        if let GroupedListItem::SessionGroup { windows, .. } = &mut items[idx] {
                            windows.push(entry);
                        }
                    } else {
                        let new_idx = items.len();
                        group_index_by_session.insert(session.clone(), new_idx);
                        items.push(GroupedListItem::SessionGroup {
                            session,
                            display_name,
                            windows: vec![entry],
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

        let mut session_matches: Vec<(&String, &String, Vec<Entry>)> = Vec::new();
        let mut standalone_matches: Vec<Entry> = Vec::new();
        let mut zoxide_matches: Vec<Entry> = Vec::new();

        for item in &self.items {
            match item {
                GroupedListItem::SessionGroup {
                    session,
                    display_name,
                    windows,
                } => {
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
                        session_matches.push((session, display_name, matched_windows));
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
        for (_session, display_name, windows) in session_matches {
            rows.push(GroupedRow::SessionHeader {
                session: display_name.clone(),
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
                GroupedListItem::SessionGroup {
                    session: _,
                    display_name,
                    windows,
                } => {
                    rows.push(GroupedRow::SessionHeader {
                        session: display_name.clone(),
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
            GroupedListItem::SessionGroup {
                session,
                display_name: _,
                windows,
            } => {
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

    #[test]
    fn e2e_vietnamese_search_with_indices() {
        // Full pipeline: Vietnamese entries → fold → nucleo match → real indices
        let snapshot = Snapshot::new(
            vec![
                Entry::zoxide("tư-vấn".into(), "/proj/tu-van".into()),
                Entry::zoxide("giải pháp".into(), "/proj/giai-phap".into()),
                Entry::zoxide("alpha".into(), "/proj/alpha".into()),
            ],
            "s".into(),
            "s:0".into(),
        );
        let grouped = GroupedList::from_snapshot(&snapshot);
        let matcher = NucleoMatcher::new();

        let rows = grouped.filtered_rows("tư", &matcher);
        let actionable: Vec<_> = rows.iter().filter(|r| r.is_actionable()).collect();
        assert_eq!(actionable.len(), 1);

        let entry = actionable[0].actionable_entry().unwrap();
        assert!(entry.display.contains("tư-vấn"));

        // Nucleo returned real indices into the display string
        assert!(!entry.matched_indices.is_empty());
        for &idx in &entry.matched_indices {
            let idx = idx as usize;
            assert!(
                idx < entry.display.chars().count(),
                "index {} out of bounds for display {:?}",
                idx,
                entry.display
            );
        }
    }

    #[test]
    fn e2e_ascii_search_finds_vietnamese_entry() {
        let snapshot = Snapshot::new(
            vec![
                Entry::zoxide("giải pháp".into(), "/proj/giai-phap".into()),
                Entry::zoxide("alpha".into(), "/proj/alpha".into()),
            ],
            "s".into(),
            "s:0".into(),
        );
        let grouped = GroupedList::from_snapshot(&snapshot);
        let matcher = NucleoMatcher::new();

        let rows = grouped.filtered_rows("giai", &matcher);
        let actionable: Vec<_> = rows.iter().filter(|r| r.is_actionable()).collect();
        assert_eq!(actionable.len(), 1);

        let entry = actionable[0].actionable_entry().unwrap();
        assert!(entry.display.contains("giải pháp"));
        assert!(!entry.matched_indices.is_empty());
    }

    #[test]
    fn single_session_multi_window_shows_basename_not_path() {
        // Session "public-api" with 2 windows same basename → display just "public-api"
        let snapshot = Snapshot::new(
            vec![
                Entry::window(
                    "public-api".into(),
                    "0".into(),
                    "editor".into(),
                    "/Projects/public-api".into(),
                    SortPriority::CurrentWindow,
                    true,
                    None,
                    None,
                ),
                Entry::window(
                    "public-api".into(),
                    "1".into(),
                    "shell".into(),
                    "/Projects/public-api".into(),
                    SortPriority::CurrentSessionOtherWindow,
                    false,
                    None,
                    None,
                ),
            ],
            "public-api".into(),
            "public-api:0".into(),
        );
        let grouped = GroupedList::from_snapshot(&snapshot);
        match &grouped.items[0] {
            GroupedListItem::SessionGroup {
                display_name,
                windows,
                ..
            } => {
                assert_eq!(display_name, "public-api");
                assert_eq!(windows.len(), 2);
            }
            other => panic!("expected SessionGroup, got {other:?}"),
        }
    }

    #[test]
    fn two_sessions_same_basename_disambiguate_with_parent() {
        // Two different sessions with same basename "public" → show parent/basename
        let snapshot = Snapshot::new(
            vec![
                Entry::window(
                    "public".into(),
                    "0".into(),
                    "work".into(),
                    "/henull2/public".into(),
                    SortPriority::OtherSessionWindow,
                    false,
                    None,
                    None,
                ),
                Entry::window(
                    "public-1".into(),
                    "0".into(),
                    "other".into(),
                    "/henullcom/public".into(),
                    SortPriority::OtherSessionWindow,
                    false,
                    None,
                    None,
                ),
            ],
            "s".into(),
            "s:0".into(),
        );
        let grouped = GroupedList::from_snapshot(&snapshot);

        // Both are standalone (1 window each)
        let displays: Vec<&str> = grouped
            .items
            .iter()
            .filter_map(|item| match item {
                GroupedListItem::StandaloneSession(e) => Some(e.display.as_str()),
                _ => None,
            })
            .collect();

        assert_eq!(displays.len(), 2);
        assert!(
            displays[0].contains("henull2/public"),
            "expected henull2/public in {:?}",
            displays[0]
        );
        assert!(
            displays[1].contains("henullcom/public"),
            "expected henullcom/public in {:?}",
            displays[1]
        );
    }

    #[test]
    fn two_sessions_same_path_appends_session_name() {
        // Two sessions at identical path → display appends session name
        let snapshot = Snapshot::new(
            vec![
                Entry::window(
                    "work".into(),
                    "0".into(),
                    "editor".into(),
                    "/project".into(),
                    SortPriority::OtherSessionWindow,
                    false,
                    None,
                    None,
                ),
                Entry::window(
                    "side".into(),
                    "0".into(),
                    "shell".into(),
                    "/project".into(),
                    SortPriority::OtherSessionWindow,
                    false,
                    None,
                    None,
                ),
            ],
            "s".into(),
            "s:0".into(),
        );
        let grouped = GroupedList::from_snapshot(&snapshot);

        let displays: Vec<&str> = grouped
            .items
            .iter()
            .filter_map(|item| match item {
                GroupedListItem::StandaloneSession(e) => Some(e.display.as_str()),
                _ => None,
            })
            .collect();

        assert_eq!(displays.len(), 2);
        assert!(
            displays[0].contains("project (work)"),
            "expected 'project (work)' in {:?}",
            displays[0]
        );
        assert!(
            displays[1].contains("project (side)"),
            "expected 'project (side)' in {:?}",
            displays[1]
        );
    }
}
