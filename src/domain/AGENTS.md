# DOMAIN LAYER

**Generated:** 2026-04-05  **Commit:** de29aaf  **Branch:** main

Core business logic, entities, and domain rules. No external dependencies, no I/O.
8 modules.

## STRUCTURE
| File | Purpose | Lines |
|------|---------|-------|
| `entry.rs` | Entry entity: Window or Zoxide entry, sorting priority | 188 |
| `sort.rs` | SortPriority logic: Current → Session → Other → Zoxide | 195 |
| `grouped_list.rs` | Group entries by session, filter logic, GroupedRow | 191 |
| `action.rs` | Action enum: Goto, Kill, TogglePreview, Reload, Quit | 104 |
| `error.rs` | AdapterError, ActionError domain error types | 130 |
| `snapshot.rs` | Snapshot: entries + current session/window | 65 |
| `session.rs` | Simple Session name container | — |
| `path_name.rs` | basename_from_path() utility | 28 |

## KEY TYPES

```rust
// entry.rs
pub enum EntryType { Window, Zoxide }
pub enum SortPriority { CurrentWindow, CurrentSessionOtherWindow, OtherSessionWindow, ZoxideDirectory }
pub struct Entry { entry_type, display, target, path, priority, session_name, is_current }

// action.rs
pub enum Action { Goto { target, path, entry_type }, Kill { target, entry_type }, TogglePreview, Reload, Quit }

// snapshot.rs
pub struct Snapshot { entries, current_session, current_window }

// grouped_list.rs
pub struct GroupedList // session-grouped entries with filter
pub struct GroupedRow  // header or entry row, is_actionable(), actionable_entry()

// path_name.rs
pub fn basename_from_path(path: &str) -> String
```

## SORTING RULES (sort.rs)
1. Current window first
2. Other windows in current session
3. Windows from other sessions
4. Zoxide directories last

## ANTI-PATTERNS
- NEVER add I/O here (keep domain pure)
- NEVER use external crate types in public API
