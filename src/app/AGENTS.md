# APPLICATION LAYER

**Generated:** 2026-04-05  **Commit:** de29aaf  **Branch:** main

Orchestrates the TUI. Controller loop, state management, action execution.
10 modules after controller refactor.

## STRUCTURE
| File | Purpose | Lines |
|------|---------|-------|
| `controller.rs` | Main event loop, delegates to submodules | 503 |
| `state.rs` | AppState: full application state machine | 732 |
| `executor.rs` | ActionExecutor: executes actions via tmux | 302 |
| `loader.rs` | SnapshotLoader: loads tmux + zoxide data | 228 |
| `events.rs` | Key mapping + action application | 260 |
| `event_action_coordinator.rs` | LoopOutcome enum, handle_action() | 82 |
| `preview_orchestrator.rs` | PreviewRequestTracker, 250ms debounce | 100 |
| `state_helpers.rs` | Selection/index helpers for GroupedRow | 105 |
| `terminal_lifecycle.rs` | init_terminal(), restore_terminal() | 18 |
| `tmux_window_mapper.rs` | map_raw_windows_to_entries() | 58 |
| `mod.rs` | Module exports | — |

## KEY COMPONENTS

```rust
// controller.rs — main loop, delegates to submodules
pub struct AppController { state, loader, tmux, preview_loader, tracker }

// state.rs — full app state machine
pub struct AppState {
    snapshot, grouped_list, selected_index, selected_target,
    filter, preview_visible, preview_state, status_message, should_quit,
}

// event_action_coordinator.rs — action → outcome
pub(super) enum LoopOutcome { Continue, Quit, SwitchTo(String), ReloadSnapshot }
pub(super) fn handle_action(action, state, tmux) -> Result<LoopOutcome>

// executor.rs — action execution
pub enum ExitReason { Quit, SwitchTo(String), Reload }
pub struct ActionExecutor;
impl ActionExecutor { pub fn execute(action, tmux) -> Result<ExitReason, ActionError>; }

// preview_orchestrator.rs — lazy preview trigger
pub(super) struct PreviewRequestTracker { last_preview_target, last_preview_dimensions, last_preview_requested_at }
const PREVIEW_REFRESH_INTERVAL: Duration = 250ms;

// state_helpers.rs — selection utilities
pub(super) fn actionable_count(rows) -> usize
pub(super) fn selected_actionable_entry(rows, selected_index) -> Option<Entry>
```

## LIFECYCLE
1. `run_app()` → init terminal → create controller
2. `controller.run()` → event loop
3. Handle key → `map_key_to_action()` → `handle_action()`
4. On Enter → `build_enter_action()` → `ActionExecutor::execute()`
5. On selection change → `PreviewRequestTracker::trigger_if_needed()` (250ms debounce)

## ANTI-PATTERNS
- NEVER block main thread (preview is async)
- NEVER mutate state outside event handlers
- NEVER call tmux directly (use ActionExecutor)
