# PREVIEW SUBSYSTEM

**Generated:** 2026-04-05  **Commit:** de29aaf  **Branch:** main

Async preview generation. Thread-based, lazy loading, timeout-protected.
7 modules after assembly/capture/directory extraction.

## STRUCTURE
| File | Purpose | Lines |
|------|---------|-------|
| `generator.rs` | PreviewGenerator: orchestrates preview generation | 852 |
| `loader.rs` | AsyncPreviewLoader: thread-based async loading | 379 |
| `ansi.rs` | ANSI escape sequence stripping | 226 |
| `directory.rs` | generate_directory_preview_content() | 110 |
| `pane_capture.rs` | capture_pane_lines(), tail_lines_preserve_layout() | 37 |
| `assembly.rs` | build_tmux_screen_content(), list_session_windows() | 41 |
| `types.rs` | PreviewState enum + content structs | 52 |
| `mod.rs` | Module exports | — |

## KEY TYPES

```rust
// types.rs
pub enum PreviewState { TmuxScreen(TmuxScreenContent), DirectoryListing(DirectoryListingContent), Loading, Error(String), Empty }
pub struct TmuxScreenContent { session_name, path, target, windows, screen_lines, is_fallback }
pub struct DirectoryListingContent { name, path, headline, entries, has_session, source }

// generator.rs
pub struct PreviewGenerator { tmux, factory }
impl PreviewGenerator { pub fn generate(entry, dimensions) -> Result<PreviewState, String>; }

// loader.rs
pub struct AsyncPreviewLoader { generator, receiver, current_id, current_target, current_dimensions }

// pane_capture.rs
pub fn capture_pane_lines(tmux, entry, dimensions, max_pane_lines) -> Result<Vec<String>, AdapterError>

// directory.rs
pub fn generate_directory_preview_content(tmux, entry, source, max_dir_entries) -> Result<DirectoryListingContent, String>

// assembly.rs
pub fn build_tmux_screen_content(entry, windows, screen_lines) -> TmuxScreenContent
pub fn list_session_windows(tmux, entry) -> Vec<String>
```

## CONSTANTS
- `PREVIEW_TIMEOUT`: 2 seconds
- `MAX_PANE_LINES`: 100
- `MAX_DIR_ENTRIES`: 15

## ASYNC FLOW
1. `request(entry, dimensions)` → spawn thread with ID
2. Thread calls `generator.generate()` with 2s timeout
3. `poll()` → non-blocking check receiver
4. Discard stale results (ID/target/dimensions mismatch)

## ANTI-PATTERNS
- NEVER block main thread on preview generation
- NEVER cache previews indefinitely (250ms debounce refresh)
- NEVER panic on tmux capture errors (graceful fallback)
