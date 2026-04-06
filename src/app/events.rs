use crate::app::state::AppState;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HandledAction {
    Quit,
    Goto,
    Kill,
    TogglePreview,
    MoveUp,
    MoveDown,
    MoveTop,
    MoveBottom,
    FilterChar(char),
    Backspace,
    ClearFilter,
    FilterCursorLeft,
    FilterCursorRight,
    Reload,
    None,
}

fn is_plain_enter(key: &KeyEvent) -> bool {
    key.modifiers == KeyModifiers::NONE && key.code == KeyCode::Enter
}

pub fn map_key_to_action(key: KeyEvent) -> HandledAction {
    if is_plain_enter(&key) {
        return HandledAction::Goto;
    }

    match (key.modifiers, key.code) {
        (KeyModifiers::CONTROL, KeyCode::Char('c')) | (KeyModifiers::NONE, KeyCode::Esc) => {
            HandledAction::Quit
        }
        (KeyModifiers::CONTROL, KeyCode::Char('d')) => HandledAction::Kill,
        (KeyModifiers::CONTROL, KeyCode::Char('o')) => HandledAction::TogglePreview,
        (KeyModifiers::CONTROL, KeyCode::Char('r')) => HandledAction::Reload,
        (KeyModifiers::NONE, KeyCode::Up) => HandledAction::MoveUp,
        (KeyModifiers::NONE, KeyCode::Down) => HandledAction::MoveDown,
        (KeyModifiers::NONE, KeyCode::Home) => HandledAction::MoveTop,
        (KeyModifiers::NONE, KeyCode::End) => HandledAction::MoveBottom,
        (KeyModifiers::NONE, KeyCode::Backspace) => HandledAction::Backspace,
        (KeyModifiers::NONE, KeyCode::Left) => HandledAction::FilterCursorLeft,
        (KeyModifiers::NONE, KeyCode::Right) => HandledAction::FilterCursorRight,
        (KeyModifiers::NONE, KeyCode::Char(c)) => HandledAction::FilterChar(c),
        _ => HandledAction::None,
    }
}

pub fn apply_action(state: &mut AppState, action: HandledAction) {
    match action {
        HandledAction::Quit => state.quit(),
        HandledAction::Goto | HandledAction::Kill | HandledAction::Reload => {}
        HandledAction::TogglePreview => state.toggle_preview(),
        HandledAction::MoveUp => state.move_selection_up(),
        HandledAction::MoveDown => state.move_selection_down(),
        HandledAction::MoveTop => state.move_selection_top(),
        HandledAction::MoveBottom => state.move_selection_bottom(),
        HandledAction::FilterChar(c) => state.set_filter(c),
        HandledAction::Backspace => state.backspace_filter(),
        HandledAction::ClearFilter => state.clear_filter(),
        HandledAction::FilterCursorLeft => state.move_filter_cursor_left(),
        HandledAction::FilterCursorRight => state.move_filter_cursor_right(),
        HandledAction::None => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    fn ctrl_char(c: char) -> KeyEvent {
        KeyEvent::new(KeyCode::Char(c), KeyModifiers::CONTROL)
    }

    #[test]
    fn enter_maps_to_goto() {
        assert_eq!(map_key_to_action(key(KeyCode::Enter)), HandledAction::Goto);
    }

    #[test]
    fn ctrl_d_maps_to_kill() {
        assert_eq!(map_key_to_action(ctrl_char('d')), HandledAction::Kill);
    }

    #[test]
    fn ctrl_o_maps_to_toggle_preview() {
        assert_eq!(
            map_key_to_action(ctrl_char('o')),
            HandledAction::TogglePreview
        );
    }

    #[test]
    fn esc_maps_to_quit() {
        assert_eq!(map_key_to_action(key(KeyCode::Esc)), HandledAction::Quit);
    }

    #[test]
    fn ctrl_c_maps_to_quit() {
        assert_eq!(map_key_to_action(ctrl_char('c')), HandledAction::Quit);
    }

    #[test]
    fn up_arrow_maps_to_move_up() {
        assert_eq!(map_key_to_action(key(KeyCode::Up)), HandledAction::MoveUp);
    }

    #[test]
    fn down_arrow_maps_to_move_down() {
        assert_eq!(
            map_key_to_action(key(KeyCode::Down)),
            HandledAction::MoveDown
        );
    }

    #[test]
    fn home_maps_to_move_top() {
        assert_eq!(
            map_key_to_action(key(KeyCode::Home)),
            HandledAction::MoveTop
        );
    }

    #[test]
    fn end_maps_to_move_bottom() {
        assert_eq!(
            map_key_to_action(key(KeyCode::End)),
            HandledAction::MoveBottom
        );
    }

    #[test]
    fn backspace_maps_correctly() {
        assert_eq!(
            map_key_to_action(key(KeyCode::Backspace)),
            HandledAction::Backspace
        );
    }

    #[test]
    fn printable_char_maps_to_filter() {
        let action = map_key_to_action(key(KeyCode::Char('x')));
        assert_eq!(action, HandledAction::FilterChar('x'));
    }

    #[test]
    fn left_right_arrow_map_to_filter_cursor_actions() {
        assert_eq!(
            map_key_to_action(key(KeyCode::Left)),
            HandledAction::FilterCursorLeft
        );
        assert_eq!(
            map_key_to_action(key(KeyCode::Right)),
            HandledAction::FilterCursorRight
        );
    }

    #[test]
    fn ctrl_r_maps_to_reload() {
        assert_eq!(map_key_to_action(ctrl_char('r')), HandledAction::Reload);
    }

    #[test]
    fn apply_move_up() {
        let snap = crate::domain::snapshot::Snapshot::new(
            vec![
                crate::domain::entry::Entry::window(
                    "s".into(),
                    "0".into(),
                    "a".into(),
                    "/".into(),
                    crate::domain::entry::SortPriority::CurrentWindow,
                    true,
                ),
                crate::domain::entry::Entry::window(
                    "s".into(),
                    "1".into(),
                    "b".into(),
                    "/".into(),
                    crate::domain::entry::SortPriority::CurrentSessionOtherWindow,
                    false,
                ),
            ],
            "s".into(),
            "s:0".into(),
        );
        let mut state = AppState::new(snap);
        state.selected_index = 1;
        apply_action(&mut state, HandledAction::MoveUp);
        assert_eq!(state.selected_index, 0);
    }

    #[test]
    fn apply_toggle_preserves_selection() {
        let snap = crate::domain::snapshot::Snapshot::new(vec![], "s".into(), "s:0".into());
        let mut state = AppState::new(snap);
        state.selected_index = 5;
        apply_action(&mut state, HandledAction::TogglePreview);
        assert_eq!(state.selected_index, 5);
        assert!(!state.preview_visible);
    }

    #[test]
    fn apply_quit() {
        let snap = crate::domain::snapshot::Snapshot::new(vec![], "s".into(), "s:0".into());
        let mut state = AppState::new(snap);
        apply_action(&mut state, HandledAction::Quit);
        assert!(state.should_quit);
    }

    #[test]
    fn apply_filter_char() {
        let snap = crate::domain::snapshot::Snapshot::new(
            vec![crate::domain::entry::Entry::window(
                "s".into(),
                "0".into(),
                "alpha".into(),
                "/".into(),
                crate::domain::entry::SortPriority::OtherSessionWindow,
                false,
            )],
            "s".into(),
            "s:0".into(),
        );
        let mut state = AppState::new(snap);
        apply_action(&mut state, HandledAction::FilterChar('a'));
        assert_eq!(state.filter, "a");
    }
}
