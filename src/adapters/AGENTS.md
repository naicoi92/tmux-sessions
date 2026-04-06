# ADAPTERS LAYER

**Generated:** 2026-04-05  **Commit:** de29aaf  **Branch:** main

External system integrations. All I/O happens here. DI-friendly via traits.
tmux/ subdirectory split into 6 focused modules.

## STRUCTURE
| File | Purpose | Lines |
|------|---------|-------|
| `tmux.rs` | TmuxSource trait definition + re-exports + inline tests | 308 |
| `tmux/` | **Subdirectory** — see `tmux/AGENTS.md` for details | — |
| `zoxide.rs` | Zoxide CLI integration + ZoxideSource trait | 165 |
| `mod.rs` | Module exports | — |

## KEY TRAITS

```rust
// tmux.rs — trait definition
pub trait TmuxSource {
    fn list_windows(&self) -> Result<Vec<RawWindow>, AdapterError>;
    fn list_sessions(&self) -> Result<Vec<RawSession>, AdapterError>;
    fn current_session(&self) -> Result<String, AdapterError>;
    fn current_window_index(&self) -> Result<String, AdapterError>;
    fn has_session(&self, name: &str) -> Result<bool, AdapterError>;
    fn select_window(&self, target: &str) -> Result<(), ActionError>;
    fn new_session(&self, name: &str, path: &str) -> Result<(), ActionError>;
    fn new_window(&self, session: &str, path: &str) -> Result<String, ActionError>;
    fn switch_client(&self, target: &str) -> Result<(), ActionError>;
    fn kill_window(&self, target: &str) -> Result<(), ActionError>;
    fn kill_session(&self, name: &str) -> Result<(), ActionError>;
    fn capture_pane(&self, target: &str, line_count: usize) -> Result<String, AdapterError>;
    fn capture_pane_with_size(&self, target, line_count, width, height) -> Result<String, AdapterError>;
}

// zoxide.rs
pub trait ZoxideSource {
    fn query(&self, limit: usize) -> Result<Vec<String>, AdapterError>;
    fn directories(&self, limit: usize) -> Result<Vec<Entry>, AdapterError>;
}
```

## TEST DOUBLES
- `FakeTmuxSource` (tmux/fake.rs): Configurable mock with `fail_on` field
- `FakeTmuxCall` enum: tracked calls for assertions
- No FakeZoxideSource — tests use default ZoxideAdapter

## ANTI-PATTERNS
- NEVER use std::process::Command outside this layer
- NEVER block on long-running tmux commands in main thread
