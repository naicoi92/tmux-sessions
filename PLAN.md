# Kế hoạch đơn giản hóa `src/app/executor.rs`

## Context

- Yêu cầu: đơn giản hóa `src/app/executor.rs`, giữ nguyên hành vi, public API, và test hiện có.
- Scope sửa code: chỉ `src/app/executor.rs`.
- Luồng liên quan đã kiểm tra: `HandledAction` → `AppState::build_enter_action()`/`build_kill_action()` → `ActionExecutor` → `TmuxSource` → tests.
- Project: Rust TUI tmux, DI qua `TmuxSource`, test inline + integration, `clippy` warnings = errors.

## Approach

- Sửa trực tiếp các điểm thừa trong `execute_zoxide_goto()` và `resolve_zoxide_action()`.
- Giữ nguyên signatures public: `ActionExecutor::execute`, `extract_session_name`, `sanitize_session_name`, `resolve_zoxide_action`, `resolve_session_name`.
- Không đổi semantics: zoxide trùng path switch window; zoxide không trùng path tạo session mới; lỗi `list_sessions()` fallback qua `has_session`; utility actions vẫn trả `ExitReason::Quit`.

## Files to modify

- `src/app/executor.rs`

## Reuse

- `basename_from_path()` trong `src/domain/path_name.rs` cho trích basename.
- `Action` constructors trong `src/domain/action.rs`: `goto_window`, `goto_zoxide`, `kill_window`, `kill_zoxide`.
- `AppState::build_enter_action()` / `build_kill_action()` trong `src/app/state.rs` là nguồn tạo action cho executor.
- `handle_action()` trong `src/app/event_action_coordinator.rs` cho biết executor chủ yếu xử lý `Goto`/`Kill`; reload/toggle do coordinator/state xử lý trực tiếp.
- `TmuxSource` methods: `select_window`, `switch_client`, `list_windows`, `list_sessions`, `new_session`, `has_session`, `kill_window`, `kill_session`.
- Existing tests trong module `#[cfg(test)]` của `executor.rs`, `tests/action_executor_test.rs`, `tests/integration_test.rs`.

## Findings

- `execute_zoxide_goto()` có biến `base` và `_name` không dùng thực sự; gọi `resolve_session_name()` hai lần, lần đầu bị bỏ.
- Comment trong `execute_zoxide_goto()` nói `resolve_session_name` expects path, trong khi call bị lẫn base/path; cần làm rõ bằng một call duy nhất.
- Doc comment fallback bị lặp dòng gần giống nhau.
- `resolve_zoxide_action()` có loop thủ công tìm window trùng path; có thể dùng `iter().find(...)` giống `execute_zoxide_goto()` để giảm nesting và nhất quán.
- `Action` gồm utility variants, nhưng `handle_action()` chỉ route `Goto`/`Kill` vào executor ở flow chính; tests vẫn gọi utility variants trực tiếp. Không đổi behavior này.
- `TmuxSource::new_window()` tồn tại nhưng zoxide create flow hiện dùng `new_session()`, được tests xác nhận; không chuyển sang `new_window()`.
- `basename_from_path()` giữ root `/` fallback; `sanitize_session_name()` biến `/` thành `_`. Không đổi behavior này.
- Integration tests trong `tests/action_executor_test.rs` xác nhận `resolve_session_name()` nhận path và dùng basename; không đổi behavior này.

## Steps

- [ ] Sửa `execute_zoxide_goto()` nhánh `Ok(s)` của `list_sessions()`:
  - giữ `existing_names: Vec<String> = s.iter().map(|s| s.session_name.clone()).collect();`
  - thay toàn bộ đoạn `base`, `_name`, comment re-check, và call thứ hai bằng:
    - `let name = resolve_session_name(path, &existing_names);`
    - `tmux.new_session(&name, path)?;`
    - `tmux.switch_client(&name)?;`
    - `Ok(ExitReason::SwitchTo(name))`
- [ ] Xóa comment `Step 1/Step 2` trong `execute_zoxide_goto()` hoặc đổi thành comment ngắn có giá trị. Khuyến nghị: bỏ hết vì code đã rõ.
- [ ] Sửa doc comment trước `execute_zoxide_goto_fallback()` thành một block duy nhất:
  - `Fallback when tmux session listing fails.`
  - `Uses has_session to find a free name, so distinct paths do not merge into one session.`
- [ ] Sửa `resolve_zoxide_action()`:
  - thay `for w in windows { if paths_match(...) { return ... } }` bằng `if let Some(window) = windows.iter().find(|window| paths_match(&window.window_path, path)) { ... }`
  - giữ format target: `format!("{}:{}", window.session_name, window.window_index)`.
- [ ] Không đổi `resolve_session_name()` dù parameter tên `path`: tests hiện xác nhận hàm nhận path hoặc bare name và dùng basename.
- [ ] Không đổi `ActionExecutor::execute()` với `TogglePreview | Reload | Quit`: tests characterization đang phụ thuộc behavior này.

## Verification

- [ ] Chạy `cargo test --all`.
- [ ] Nếu cần, chạy `cargo fmt -- --check` hoặc `cargo fmt` sau khi được phép sửa code.
- [ ] Nếu cần, chạy `cargo clippy --all-targets -- -D warnings` theo chuẩn project.
