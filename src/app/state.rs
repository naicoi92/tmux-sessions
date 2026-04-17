use crate::adapters::fuzzy::NucleoMatcher;
use crate::domain::action::Action;
use crate::domain::entry::Entry;
use crate::domain::grouped_list::{GroupedList, GroupedRow};
use crate::domain::snapshot::Snapshot;
use crate::preview::types::PreviewState;

use crate::app::state_helpers;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UiEvent {
    Key(crossterm::event::KeyEvent),
    Resize(u16, u16),
}

pub struct AppState {
    pub snapshot: Snapshot,
    grouped_list: GroupedList,
    pub selected_index: usize,
    pub selected_target: Option<String>,
    pub filter: String,
    pub filter_cursor: usize,
    pub preview_visible: bool,
    pub preview_state: PreviewState,
    pub status_message: Option<String>,
    pub should_quit: bool,
    filtered_rows_cache: Vec<GroupedRow>,
    fuzzy_matcher: NucleoMatcher,
}

impl AppState {
    pub fn new(snapshot: Snapshot) -> Self {
        let grouped_list = GroupedList::from_snapshot(&snapshot);
        let mut state = Self {
            snapshot,
            grouped_list,
            selected_index: 0,
            selected_target: None,
            filter: String::new(),
            filter_cursor: 0,
            preview_visible: true,
            preview_state: PreviewState::Empty,
            status_message: None,
            should_quit: false,
            filtered_rows_cache: Vec::new(),
            fuzzy_matcher: NucleoMatcher::new(),
        };
        state.rebuild_filtered_rows_cache();
        state.move_selection_top();
        state
    }

    pub fn filtered_entries(&self) -> Vec<Entry> {
        if self.filter.is_empty() {
            self.grouped_list.actionable_entries()
        } else {
            self.filtered_rows()
                .iter()
                .filter_map(|row| row.actionable_entry().cloned())
                .collect()
        }
    }

    pub fn grouped_list(&self) -> &GroupedList {
        &self.grouped_list
    }

    pub fn filtered_rows(&self) -> &[GroupedRow] {
        &self.filtered_rows_cache
    }

    fn rebuild_filtered_rows_cache(&mut self) {
        self.filtered_rows_cache = self
            .grouped_list
            .filtered_rows(&self.filter, &self.fuzzy_matcher);
    }

    pub fn replace_snapshot(&mut self, snapshot: Snapshot) {
        let previous_target = self.current_selected_target();
        let previous_visible_index = self.selected_visible_index();
        self.snapshot = snapshot;
        self.grouped_list = GroupedList::from_snapshot(&self.snapshot);
        self.rebuild_filtered_rows_cache();
        self.restore_selection(previous_target, previous_visible_index);
    }

    pub fn selected_entry(&self) -> Option<Entry> {
        self.selected_actionable_entry()
    }

    pub fn selected_actionable_entry(&self) -> Option<Entry> {
        let rows = self.filtered_rows();
        state_helpers::selected_actionable_entry(rows, self.selected_index)
    }

    pub fn selected_visible_index(&self) -> Option<usize> {
        let rows = self.filtered_rows();
        state_helpers::selected_visible_index(rows, self.selected_index)
    }

    pub fn move_selection_up(&mut self) {
        if self.selected_index > 0 {
            self.selected_index -= 1;
        }
        self.sync_selected_target();
    }

    pub fn move_selection_down(&mut self) {
        let max = self.actionable_count().saturating_sub(1);
        if self.selected_index < max {
            self.selected_index += 1;
        }
        self.sync_selected_target();
    }

    pub fn move_selection_top(&mut self) {
        self.selected_index = 0;
        self.sync_selected_target();
    }

    pub fn move_selection_bottom(&mut self) {
        self.selected_index = self.actionable_count().saturating_sub(1);
        self.sync_selected_target();
    }

    pub fn toggle_preview(&mut self) {
        self.preview_visible = !self.preview_visible;
    }

    pub fn set_filter(&mut self, ch: char) {
        self.insert_filter_char(ch);
    }

    pub fn backspace_filter(&mut self) {
        self.delete_filter_char();
    }

    pub fn clear_filter(&mut self) {
        self.clear_filter_with_cursor();
    }

    pub fn move_filter_cursor_left(&mut self) {
        if self.filter_cursor > 0 {
            self.filter_cursor -= 1;
        }
    }

    pub fn move_filter_cursor_right(&mut self) {
        if self.filter_cursor < self.filter.len() {
            self.filter_cursor += 1;
        }
    }

    pub fn insert_filter_char(&mut self, ch: char) {
        state_helpers::insert_filter_char(&mut self.filter, &mut self.filter_cursor, ch);
        self.rebuild_filtered_rows_cache();
        self.focus_on_best_match();
    }

    pub fn delete_filter_char(&mut self) {
        if self.filter_cursor > 0 && !self.filter.is_empty() {
            state_helpers::delete_filter_char(&mut self.filter, &mut self.filter_cursor);
            self.rebuild_filtered_rows_cache();
            self.focus_on_best_match();
        }
    }

    fn focus_on_best_match(&mut self) {
        self.move_selection_top();
    }

    pub fn clear_filter_with_cursor(&mut self) {
        let previous_target = self.current_selected_target();
        let previous_visible_index = self.selected_visible_index();

        state_helpers::clear_filter_with_cursor(&mut self.filter, &mut self.filter_cursor);

        self.rebuild_filtered_rows_cache();
        self.restore_selection(previous_target, previous_visible_index);
    }

    pub fn clamp_selection(&mut self) {
        let max = self.actionable_count().saturating_sub(1);
        if self.selected_index > max {
            self.selected_index = max;
        }
        self.sync_selected_target();
    }

    fn restore_selection(
        &mut self,
        preferred_target: Option<String>,
        anchor_visible_index: Option<usize>,
    ) {
        let (selected_index, selected_target) = state_helpers::restore_selection(
            self.filtered_rows(),
            self.selected_index,
            preferred_target,
            anchor_visible_index,
        );
        self.selected_index = selected_index;
        self.selected_target = selected_target;
    }

    fn current_selected_target(&self) -> Option<String> {
        self.selected_target
            .clone()
            .or_else(|| self.selected_actionable_entry().map(|entry| entry.target))
    }

    fn sync_selected_target(&mut self) {
        self.selected_target = self.selected_actionable_entry().map(|entry| entry.target);
    }

    fn actionable_count(&self) -> usize {
        let rows = self.filtered_rows();
        state_helpers::actionable_count(rows)
    }

    pub fn build_action(&self) -> Option<Action> {
        let entry = self.selected_actionable_entry()?;
        match entry.entry_type {
            crate::domain::entry::EntryType::Window => {
                Some(Action::goto_window(entry.target, entry.path))
            }
            crate::domain::entry::EntryType::Zoxide => {
                Some(Action::goto_zoxide(entry.target, entry.path))
            }
        }
    }

    pub fn build_enter_action(&self) -> Option<Action> {
        self.build_action()
    }

    pub fn build_kill_action(&self) -> Option<Action> {
        let entry = self.selected_actionable_entry()?;
        match entry.entry_type {
            crate::domain::entry::EntryType::Window => Some(Action::kill_window(entry.target)),
            crate::domain::entry::EntryType::Zoxide => Some(Action::kill_zoxide(entry.target)),
        }
    }

    pub fn quit(&mut self) {
        self.should_quit = true;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::entry::{EntryType, SortPriority};

    fn make_snap(entries: Vec<Entry>) -> Snapshot {
        Snapshot::new(entries, "s1".into(), "s1:0".into())
    }

    fn e_window(name: &str) -> Entry {
        Entry::window(
            "s1".into(),
            "0".into(),
            name.into(),
            "/p".into(),
            SortPriority::OtherSessionWindow,
            false,
            None,
            None,
        )
    }

    fn e_window_with_index(name: &str, index: &str) -> Entry {
        Entry::window(
            "s1".into(),
            index.into(),
            name.into(),
            "/p".into(),
            SortPriority::OtherSessionWindow,
            false,
            None,
            None,
        )
    }

    fn e_zoxide(name: &str) -> Entry {
        Entry::zoxide(name.into(), format!("/{name}"))
    }

    #[test]
    fn initial_state_selection_zero() {
        let state = AppState::new(make_snap(vec![e_window("a")]));
        assert_eq!(state.selected_index, 0);
        assert!(state.preview_visible);
        assert!(!state.should_quit);
    }

    #[test]
    fn move_down_increments() {
        let mut state = AppState::new(make_snap(vec![e_window("a"), e_window("b")]));
        state.move_selection_down();
        assert_eq!(state.selected_index, 1);
    }

    #[test]
    fn move_down_clamps_at_end() {
        let mut state = AppState::new(make_snap(vec![e_window("a")]));
        state.move_selection_down();
        assert_eq!(state.selected_index, 0);
    }

    #[test]
    fn move_up_clamps_at_zero() {
        let mut state = AppState::new(make_snap(vec![e_window("a")]));
        state.move_selection_up();
        assert_eq!(state.selected_index, 0);
    }

    #[test]
    fn move_up_decrements() {
        let mut state = AppState::new(make_snap(vec![e_window("a"), e_window("b")]));
        state.selected_index = 1;
        state.move_selection_up();
        assert_eq!(state.selected_index, 0);
    }

    #[test]
    fn move_bottom_goes_to_last() {
        let mut state = AppState::new(make_snap(vec![e_window("a"), e_window("b"), e_window("c")]));
        state.move_selection_bottom();
        assert_eq!(state.selected_index, 2);
    }

    #[test]
    fn move_top_goes_to_zero() {
        let mut state = AppState::new(make_snap(vec![e_window("a"), e_window("b")]));
        state.selected_index = 1;
        state.move_selection_top();
        assert_eq!(state.selected_index, 0);
    }

    #[test]
    fn toggle_preview_flips() {
        let mut state = AppState::new(make_snap(vec![]));
        assert!(state.preview_visible);
        state.toggle_preview();
        assert!(!state.preview_visible);
        state.toggle_preview();
        assert!(state.preview_visible);
    }

    #[test]
    fn toggle_preserves_selection() {
        let mut state = AppState::new(make_snap(vec![e_window("a"), e_window("b")]));
        state.selected_index = 1;
        state.toggle_preview();
        assert_eq!(state.selected_index, 1);
    }

    #[test]
    fn filter_narrows_entries() {
        let mut state = AppState::new(make_snap(vec![
            e_window_with_index("alpha", "0"),
            e_window_with_index("beta", "1"),
            e_zoxide("gamma"),
        ]));
        state.set_filter('b');
        assert_eq!(state.filtered_entries().len(), 1);
    }

    #[test]
    fn clear_filter_restores_all() {
        let mut state = AppState::new(make_snap(vec![
            e_window_with_index("a", "0"),
            e_window_with_index("b", "1"),
        ]));
        state.set_filter('a');
        assert_eq!(state.filtered_entries().len(), 1);
        state.clear_filter();
        assert_eq!(state.filtered_entries().len(), 2);
    }

    #[test]
    fn backspace_removes_char() {
        let mut state = AppState::new(make_snap(vec![e_window("abc")]));
        state.set_filter('a');
        state.set_filter('b');
        assert_eq!(state.filter, "ab");
        state.backspace_filter();
        assert_eq!(state.filter, "a");
    }

    #[test]
    fn filter_resets_selection() {
        let mut state = AppState::new(make_snap(vec![e_window("a"), e_window("b"), e_window("c")]));
        state.selected_index = 2;
        state.set_filter('b');
        assert_eq!(state.selected_index, 0);
    }

    #[test]
    fn clamp_selection_on_filter() {
        let mut state = AppState::new(make_snap(vec![
            e_window("alpha"),
            e_window("bravo"),
            e_window("charlie"),
        ]));
        state.selected_index = 2;
        state.set_filter('a');
        state.clamp_selection();
        assert_eq!(state.selected_index, 0);
    }

    #[test]
    fn replace_snapshot_restores_by_stable_target_when_order_changes() {
        let mut state = AppState::new(Snapshot::new(
            vec![
                Entry::window(
                    "s1".into(),
                    "0".into(),
                    "alpha".into(),
                    "/p".into(),
                    SortPriority::OtherSessionWindow,
                    false,
                    None,
                    None,
                ),
                Entry::window(
                    "s2".into(),
                    "0".into(),
                    "beta".into(),
                    "/p".into(),
                    SortPriority::OtherSessionWindow,
                    false,
                    None,
                    None,
                ),
            ],
            "s1".into(),
            "s1:0".into(),
        ));
        state.move_selection_down();
        assert_eq!(state.selected_entry().unwrap().target, "s2:0");

        let swapped = Snapshot::new(
            vec![
                Entry::window(
                    "s2".into(),
                    "0".into(),
                    "beta".into(),
                    "/p".into(),
                    SortPriority::OtherSessionWindow,
                    false,
                    None,
                    None,
                ),
                Entry::window(
                    "s1".into(),
                    "0".into(),
                    "alpha".into(),
                    "/p".into(),
                    SortPriority::OtherSessionWindow,
                    false,
                    None,
                    None,
                ),
            ],
            "s1".into(),
            "s1:0".into(),
        );

        state.replace_snapshot(swapped);
        assert_eq!(state.selected_entry().unwrap().target, "s2:0");
    }

    #[test]
    fn replace_snapshot_falls_back_to_nearest_actionable_row() {
        let mut state = AppState::new(Snapshot::new(
            vec![
                Entry::window(
                    "a".into(),
                    "0".into(),
                    "a".into(),
                    "/p".into(),
                    SortPriority::OtherSessionWindow,
                    false,
                    None,
                    None,
                ),
                Entry::window(
                    "b".into(),
                    "0".into(),
                    "b".into(),
                    "/p".into(),
                    SortPriority::OtherSessionWindow,
                    false,
                    None,
                    None,
                ),
                Entry::window(
                    "c".into(),
                    "0".into(),
                    "c".into(),
                    "/p".into(),
                    SortPriority::OtherSessionWindow,
                    false,
                    None,
                    None,
                ),
            ],
            "a".into(),
            "a:0".into(),
        ));
        state.move_selection_down();
        assert_eq!(state.selected_entry().unwrap().target, "b:0");

        let removed_selected = Snapshot::new(
            vec![
                Entry::window(
                    "a".into(),
                    "0".into(),
                    "a".into(),
                    "/p".into(),
                    SortPriority::OtherSessionWindow,
                    false,
                    None,
                    None,
                ),
                Entry::window(
                    "c".into(),
                    "0".into(),
                    "c".into(),
                    "/p".into(),
                    SortPriority::OtherSessionWindow,
                    false,
                    None,
                    None,
                ),
            ],
            "a".into(),
            "a:0".into(),
        );

        state.replace_snapshot(removed_selected);
        assert_eq!(state.selected_entry().unwrap().target, "c:0");
    }

    #[test]
    fn clear_filter_roundtrip_preserves_selected_target_identity() {
        let mut state = AppState::new(Snapshot::new(
            vec![
                Entry::window(
                    "s".into(),
                    "0".into(),
                    "alpha".into(),
                    "/p".into(),
                    SortPriority::OtherSessionWindow,
                    false,
                    None,
                    None,
                ),
                Entry::window(
                    "s".into(),
                    "1".into(),
                    "beta".into(),
                    "/p".into(),
                    SortPriority::OtherSessionWindow,
                    false,
                    None,
                    None,
                ),
                Entry::window(
                    "s".into(),
                    "2".into(),
                    "gamma".into(),
                    "/p".into(),
                    SortPriority::OtherSessionWindow,
                    false,
                    None,
                    None,
                ),
            ],
            "s".into(),
            "s:0".into(),
        ));

        state.move_selection_bottom();
        assert_eq!(state.selected_entry().unwrap().target, "s:2");

        state.set_filter('g');
        assert_eq!(state.selected_entry().unwrap().target, "s:2");

        state.clear_filter();
        assert_eq!(state.selected_entry().unwrap().target, "s:2");
    }

    #[test]
    fn selected_entry_returns_correct() {
        let state = AppState::new(make_snap(vec![e_window("first"), e_window("second")]));
        assert_eq!(state.selected_entry().unwrap().display, "  ◆ s1 [0]: first");
        let mut state = state;
        state.selected_index = 1;
        assert_eq!(
            state.selected_entry().unwrap().display,
            "  ◆ s1 [0]: second"
        );
    }

    #[test]
    fn selected_entry_none_when_empty() {
        let state = AppState::new(make_snap(vec![]));
        assert!(state.selected_entry().is_none());
    }

    #[test]
    fn build_action_for_window() {
        let state = AppState::new(make_snap(vec![e_window("main")]));
        let action = state.build_action().unwrap();
        match action {
            Action::Goto { entry_type, .. } => assert_eq!(entry_type, EntryType::Window),
            _ => panic!("expected Goto"),
        }
    }

    #[test]
    fn build_action_for_zoxide() {
        let state = AppState::new(make_snap(vec![e_zoxide("proj")]));
        let action = state.build_action().unwrap();
        match action {
            Action::Goto { entry_type, .. } => assert_eq!(entry_type, EntryType::Zoxide),
            _ => panic!("expected Goto"),
        }
    }

    #[test]
    fn build_kill_action() {
        let state = AppState::new(make_snap(vec![e_window("w")]));
        let action = state.build_kill_action().unwrap();
        match action {
            Action::Kill { entry_type, .. } => assert_eq!(entry_type, EntryType::Window),
            _ => panic!("expected Kill"),
        }
    }

    #[test]
    fn quit_sets_flag() {
        let mut state = AppState::new(make_snap(vec![]));
        state.quit();
        assert!(state.should_quit);
    }

    #[test]
    fn toggle_preview_does_not_clear_filter() {
        let mut state = AppState::new(make_snap(vec![e_window("a")]));
        state.set_filter('a');
        state.toggle_preview();
        assert_eq!(state.filter, "a");
    }

    #[test]
    fn filter_cursor_starts_at_zero() {
        let state = AppState::new(make_snap(vec![e_window("a")]));
        assert_eq!(state.filter_cursor, 0);
    }

    #[test]
    fn move_filter_cursor_left_right_within_bounds() {
        let mut state = AppState::new(make_snap(vec![e_window("alpha")]));
        state.set_filter('a');
        state.set_filter('b');

        state.move_filter_cursor_left();
        assert_eq!(state.filter_cursor, 1);

        state.move_filter_cursor_left();
        state.move_filter_cursor_left();
        assert_eq!(state.filter_cursor, 0);

        state.move_filter_cursor_right();
        assert_eq!(state.filter_cursor, 1);

        state.move_filter_cursor_right();
        state.move_filter_cursor_right();
        assert_eq!(state.filter_cursor, 2);
    }

    #[test]
    fn insert_filter_char_inserts_at_cursor_position() {
        let mut state = AppState::new(make_snap(vec![e_window("alpha")]));
        state.set_filter('a');
        state.set_filter('c');
        state.move_filter_cursor_left();

        state.insert_filter_char('b');

        assert_eq!(state.filter, "abc");
        assert_eq!(state.filter_cursor, 2);
    }

    #[test]
    fn delete_filter_char_removes_character_before_cursor() {
        let mut state = AppState::new(make_snap(vec![e_window("alpha")]));
        state.set_filter('a');
        state.set_filter('b');
        state.set_filter('c');
        state.move_filter_cursor_left();

        state.delete_filter_char();

        assert_eq!(state.filter, "ac");
        assert_eq!(state.filter_cursor, 1);
    }

    #[test]
    fn clear_filter_with_cursor_resets_filter_and_cursor() {
        let mut state = AppState::new(make_snap(vec![e_window("alpha")]));
        state.set_filter('a');
        state.set_filter('b');

        state.clear_filter_with_cursor();

        assert!(state.filter.is_empty());
        assert_eq!(state.filter_cursor, 0);
    }

    #[test]
    fn filtered_rows_cache_updates_after_filter_change() {
        let mut state = AppState::new(Snapshot::new(
            vec![
                Entry::window(
                    "s".into(),
                    "0".into(),
                    "alpha".into(),
                    "/p".into(),
                    SortPriority::CurrentWindow,
                    true,
                    None,
                    None,
                ),
                Entry::window(
                    "s".into(),
                    "1".into(),
                    "beta".into(),
                    "/p".into(),
                    SortPriority::CurrentSessionOtherWindow,
                    false,
                    None,
                    None,
                ),
            ],
            "s".into(),
            "s:0".into(),
        ));
        let before = state.filtered_rows().len();
        state.insert_filter_char('b');
        let after = state.filtered_rows().len();
        assert!(after < before);
    }

    #[test]
    fn filtered_rows_reuses_cached_vector_between_reads() {
        let state = AppState::new(Snapshot::new(
            vec![
                Entry::window(
                    "s".into(),
                    "0".into(),
                    "main".into(),
                    "/tmp".into(),
                    SortPriority::CurrentWindow,
                    true,
                    None,
                    None,
                ),
                Entry::zoxide("proj".into(), "/tmp/proj".into()),
            ],
            "s".into(),
            "s:0".into(),
        ));
        let first_ptr = state.filtered_rows().as_ptr();
        let second_ptr = state.filtered_rows().as_ptr();
        assert_eq!(first_ptr, second_ptr);
    }
}
