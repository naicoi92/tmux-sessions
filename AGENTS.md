# PROJECT KNOWLEDGE BASE

**Generated:** 2026-04-05  **Commit:** de29aaf  **Branch:** main

## OVERVIEW
TUI popup manager for tmux sessions and zoxide directories. Rust + ratatui. Single binary, no daemon, direct tmux CLI calls.

## STRUCTURE
```
tmux-sessions/
├── src/
│   ├── domain/         # Core types, entities, sorting (pure, no I/O)
│   ├── adapters/       # External integrations: tmux/, zoxide.rs
│   │   └── tmux/       # Split: command, parser, capture, raw, fake, adapter_impl
│   ├── app/            # Controller, state machine, event loop, executor (10 modules)
│   ├── preview/        # Async preview: generator, loader, assembly, pane_capture, directory
│   └── ui/             # ratatui rendering + theme system
├── tests/              # Integration + E2E tests
├── scripts/verify.sh   # Local fmt + clippy + test + release build
└── Cargo.toml          # autobins=false, explicit [[bin]]
```

## WHERE TO LOOK
| Task | Location | Notes |
|------|----------|-------|
| Add new entry type | `src/domain/entry.rs` | EntryType enum, Entry struct |
| Modify sorting | `src/domain/sort.rs` | SortPriority logic |
| Path utilities | `src/domain/path_name.rs` | basename_from_path() |
| Change tmux interaction | `src/adapters/tmux.rs` | TmuxSource trait definition |
| Add tmux command | `src/adapters/tmux/command.rs` | run_tmux(), run_capture_pane() |
| Parse tmux output | `src/adapters/tmux/parser.rs` | parse_windows(), parse_sessions() |
| Pane capture logic | `src/adapters/tmux/capture.rs` | capture_best_effort(), dual-pane |
| Add DI fake | `src/adapters/tmux/fake.rs` | FakeTmuxSource, FakeTmuxCall |
| Modify keybindings | `src/app/events.rs` | map_key_to_action() |
| Change state machine | `src/app/state.rs` | AppState + state_helpers.rs |
| Preview orchestration | `src/app/preview_orchestrator.rs` | PreviewRequestTracker, 250ms debounce |
| Execute actions | `src/app/executor.rs` | ActionExecutor → ExitReason |
| Preview content | `src/preview/generator.rs` | PreviewGenerator::generate() |
| Preview assembly | `src/preview/assembly.rs` | build_tmux_screen_content() |
| Pane capture | `src/preview/pane_capture.rs` | capture_pane_lines() |
| Directory preview | `src/preview/directory.rs` | generate_directory_preview_content() |
| UI theme/colors | `src/ui/theme.rs` | Tailwind-inspired palette, semantic styles |
| UI rendering | `src/ui/list.rs`, `src/ui/preview.rs` | ratatui widgets |
| Add test | `tests/` | Integration tests |

## CODE MAP

### Core Types
| Symbol | Type | Location | Role |
|--------|------|----------|------|
| Entry | Struct | domain/entry.rs | Session/window or zoxide dir |
| EntryType | Enum | domain/entry.rs | Window vs Zoxide |
| SortPriority | Enum | domain/entry.rs | CurrentWindow, CurrentSessionOtherWindow, etc. |
| Action | Enum | domain/action.rs | Goto, Kill, TogglePreview, Reload, Quit |
| AppState | Struct | app/state.rs | Full application state |
| Snapshot | Struct | domain/snapshot.rs | Entries + current session/window |
| GroupedList | Struct | domain/grouped_list.rs | Session-grouped entries with filter |

### Adapters (DI Layer)
| Symbol | Type | Location | Role |
|--------|------|----------|------|
| TmuxSource | Trait | adapters/tmux.rs | Abstract tmux operations |
| TmuxAdapter | Struct | adapters/tmux/adapter_impl.rs | Production tmux impl |
| FakeTmuxSource | Struct | adapters/tmux/fake.rs | Configurable test double |
| FakeTmuxCall | Enum | adapters/tmux/fake.rs | Tracked mock calls |
| RawWindow | Struct | adapters/tmux/raw.rs | Parsed tmux window data |
| RawSession | Struct | adapters/tmux/raw.rs | Parsed tmux session data |
| ZoxideSource | Trait | adapters/zoxide.rs | Abstract zoxide operations |
| ZoxideAdapter | Struct | adapters/zoxide.rs | Production zoxide impl |

### Application Layer
| Symbol | Type | Location | Role |
|--------|------|----------|------|
| AppController | Struct | app/controller.rs | Main event loop, delegates to submodules |
| LoopOutcome | Enum | app/event_action_coordinator.rs | Continue, Quit, SwitchTo, ReloadSnapshot |
| ActionExecutor | Struct | app/executor.rs | Executes actions via tmux → ExitReason |
| SnapshotLoader | Struct | app/loader.rs | Loads tmux + zoxide data |
| PreviewRequestTracker | Struct | app/preview_orchestrator.rs | Lazy preview trigger with 250ms debounce |
| map_key_to_action | Fn | app/events.rs | Key → HandledAction mapping |

### Preview Subsystem
| Symbol | Type | Location | Role |
|--------|------|----------|------|
| AsyncPreviewLoader | Struct | preview/loader.rs | Thread-based async loader |
| PreviewGenerator | Struct | preview/generator.rs | Orchestrates preview generation |
| PreviewState | Enum | preview/types.rs | Loading, TmuxScreen, DirectoryListing, Error, Empty |

## CONVENTIONS
- **Explicit binary**: `autobins = false` + explicit `[[bin]]` in Cargo.toml
- **DI pattern**: All external calls via traits (TmuxSource, ZoxideSource), fakes for tests
- **Domain errors**: AdapterError, ActionError in domain/error.rs
- **Thread safety**: Preview uses std::sync::mpsc + spawn per request
- **Tests inline**: `#[cfg(test)]` modules + integration tests in tests/
- **Formatting**: 100-char width, 4-space indent (rustfmt.toml)
- **Clippy**: cognitive complexity threshold 30 (clippy.toml), warnings as errors

## ANTI-PATTERNS (THIS PROJECT)
- NEVER block main thread for preview (always async via channels)
- NEVER use tmux temp-file bridge (direct CLI calls only)
- NEVER panic on tmux errors (graceful fallback to defaults)
- NEVER use std::process::Command directly outside adapters layer
- NEVER add I/O in domain/ (keep pure)

## UNIQUE STYLES
- **Vietnamese comments allowed**: Graceful fallback comments use Vietnamese
- **Session name sanitization**: basename_from_path() with `_` replacement, `.` prefix blocked
- **Dual-pane capture**: Tries primary + alternate screen, scores visible content, picks best
- **Zoxide limit**: DEFAULT_ZOXIDE_LIMIT = 100 entries
- **Tailwind palette**: theme.rs uses slate-900/800/700 backgrounds, semantic color constants

## COMMANDS
```bash
cargo build                     # Dev
cargo build --release           # Release
cargo run -- --debug            # Debug mode (no tmux calls)
cargo test --all                # Test all
./scripts/verify.sh             # fmt + clippy + test + release build
```

## NOTES
- Requires tmux 3.2+ for display-popup support
- Preview timeout: 2 seconds (preview/loader.rs)
- Preview refresh: 250ms debounce (app/preview_orchestrator.rs)
- Max pane lines: 100, max dir entries: 15 (preview/generator.rs)
- CI: macOS-latest, stable Rust, clippy warnings = errors
- 10 app modules after controller refactor (event_action_coordinator, preview_orchestrator, state_helpers, terminal_lifecycle, tmux_window_mapper)
