use crate::adapters::tmux::TmuxSource;
use crate::app::events::{apply_action, HandledAction};
use crate::app::executor::{ActionExecutor, ExitReason};
use crate::app::state::AppState;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum LoopOutcome {
    Continue,
    Quit,
    SwitchTo(String),
    ReloadSnapshot,
}

pub(super) fn handle_action(
    action: HandledAction,
    state: &mut AppState,
    tmux: &dyn TmuxSource,
) -> Result<LoopOutcome, Box<dyn std::error::Error>> {
    let outcome = match action {
        HandledAction::Quit => {
            apply_action(state, action);
            LoopOutcome::Quit
        }
        HandledAction::Goto => {
            if let Some(goto) = state.build_enter_action() {
                match ActionExecutor::execute(&goto, tmux) {
                    Ok(ExitReason::SwitchTo(target)) => LoopOutcome::SwitchTo(target),
                    Ok(ExitReason::Quit) => LoopOutcome::Quit,
                    Ok(ExitReason::Reload) => LoopOutcome::ReloadSnapshot,
                    Err(e) => {
                        state.status_message = Some(format!("goto error: {e}"));
                        LoopOutcome::Continue
                    }
                }
            } else {
                LoopOutcome::Continue
            }
        }
        HandledAction::Kill => {
            if let Some(kill) = state.build_kill_action() {
                match ActionExecutor::execute(&kill, tmux) {
                    Ok(_) => return Ok(LoopOutcome::ReloadSnapshot),
                    Err(e) => {
                        state.status_message = Some(format!("kill error: {e}"));
                    }
                }
            }
            LoopOutcome::Continue
        }
        HandledAction::Reload => LoopOutcome::ReloadSnapshot,
        HandledAction::FilterChar(c) => {
            state.insert_filter_char(c);
            state.clamp_selection();
            LoopOutcome::Continue
        }
        HandledAction::Backspace => {
            state.delete_filter_char();
            state.clamp_selection();
            LoopOutcome::Continue
        }
        HandledAction::ClearFilter => {
            state.clear_filter_with_cursor();
            state.clamp_selection();
            LoopOutcome::Continue
        }
        HandledAction::FilterCursorLeft => {
            state.move_filter_cursor_left();
            LoopOutcome::Continue
        }
        HandledAction::FilterCursorRight => {
            state.move_filter_cursor_right();
            LoopOutcome::Continue
        }
        _ => {
            apply_action(state, action);
            state.clamp_selection();
            LoopOutcome::Continue
        }
    };

    Ok(outcome)
}
