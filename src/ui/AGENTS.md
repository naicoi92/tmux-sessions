# UI LAYER

**Generated:** 2026-04-05  **Commit:** de29aaf  **Branch:** main

ratatui rendering + Tailwind-inspired theme system. List view, preview panel, filter bar.

## STRUCTURE
| File | Purpose | Lines |
|------|---------|-------|
| `preview.rs` | Preview panel rendering (tmux screen + directory) | 244 |
| `list.rs` | List rendering with session grouping | 136 |
| `theme.rs` | Tailwind-inspired palette, semantic colors, layout helpers | 168 |
| `mod.rs` | render(), render_help_bar() | 91 |

## RENDER FUNCTIONS

```rust
// list.rs
pub fn render_list(frame: &mut Frame, area: Rect, state: &AppState);
pub fn render_filter_bar(frame: &mut Frame, area: Rect, state: &AppState);

// preview.rs
pub fn render_preview(frame: &mut Frame, area: Rect, state: &AppState);

// mod.rs
pub fn render(frame: &mut Frame, state: &AppState);
pub fn render_help_bar(frame: &mut Frame, area: Rect);

// theme.rs — layout helpers
pub fn layout::main_layout(area: Rect, preview_visible: bool) -> Rc<[Rect]>
pub fn layout::preview_layout(area: Rect) -> Rc<[Rect]>
```

## THEME SYSTEM (theme.rs)
Tailwind-inspired dark palette:
- Backgrounds: slate-900/800/700
- Text: slate-50/300/400/500
- Accents: cyan-400, blue-400, green-400, yellow-400, red-400
- Borders: slate-600, cyan-400 (focused), blue-400 (active)
- Semantic styles module: pre-built Style constants for consistent rendering

## VISUAL ELEMENTS

**List**: Session headers `▼ session (N)` cyan bold, windows `↳ name`, zoxide `📁 name` blue, current window yellow
**Preview**: Window (session/path/status/terminal output), Directory (name/path/file listing), scroll to bottom, ANSI preserved
**Filter Bar**: `Filter: /pattern` green

## ANTI-PATTERNS
- NEVER hardcode colors (use theme constants)
- NEVER bypass state for rendering data
- NEVER block in render functions
