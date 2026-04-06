# TMUX ADAPTER SUBMODULE

**Generated:** 2026-04-05  **Commit:** de29aaf  **Branch:** main

Split from monolithic tmux.rs into 6 focused modules.
All tmux CLI interaction goes through here.

## STRUCTURE
| File | Purpose | Lines |
|------|---------|-------|
| `adapter_impl.rs` | TmuxAdapter: production TmuxSource implementation | 188 |
| `capture.rs` | Dual-pane capture: capture_best_effort(), content scoring | 182 |
| `fake.rs` | FakeTmuxSource + FakeTmuxCall for tests | 159 |
| `command.rs` | run_tmux(), run_capture_pane() — std::process::Command wrappers | 50 |
| `parser.rs` | parse_windows(), parse_sessions() — tab-separated output | 44 |
| `raw.rs` | RawWindow, RawSession data structs | 13 |

## KEY FUNCTIONS

```rust
// command.rs — low-level CLI wrappers (ONLY place with std::process::Command)
pub fn run_tmux(args: &[&str]) -> Result<String, AdapterError>
pub fn run_capture_pane(args: &[String]) -> Result<String, AdapterError>

// parser.rs — tab-separated tmux output parsing
pub fn parse_windows(output: &str) -> Result<Vec<RawWindow>, AdapterError>
pub fn parse_sessions(output: &str) -> Result<Vec<RawSession>, AdapterError>

// capture.rs — dual-pane capture with content scoring
pub fn capture_pane_args(target, line_count, alternate_screen) -> Vec<String>
pub fn select_capture_content(primary, alternate) -> Result<String, AdapterError>
pub fn capture_best_effort(target, line_count) -> Result<String, AdapterError>
pub fn capture_best_effort_with_size(target, line_count, width, height) -> Result<String, AdapterError>

// raw.rs — parsed data types
pub struct RawWindow { session_name, window_index, window_name, window_path }
pub struct RawSession { session_name, attached }

// fake.rs — test double
pub struct FakeTmuxSource { windows, sessions, current_session_name, current_window_idx, existing_sessions, fail_on }
pub enum FakeTmuxCall { SelectWindow, NewSession, SwitchClient, KillWindow, KillSession }
```

## DUAL-PANE CAPTURE LOGIC
1. Try primary screen capture
2. Try alternate screen capture (-a flag)
3. Score both by visible content
4. Prefer primary unless alternate has significantly more content

## ANTI-PATTERNS
- NEVER add std::process::Command outside command.rs
- NEVER block on capture — keep timeouts reasonable
