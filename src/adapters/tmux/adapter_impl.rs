use crate::adapters::tmux::capture::{capture_best_effort, capture_best_effort_with_size};
use crate::adapters::tmux::command::run_tmux;
use crate::adapters::tmux::parser::{parse_sessions, parse_windows};
use crate::adapters::tmux::raw::{RawSession, RawWindow};
use crate::adapters::tmux::TmuxSource;
use crate::domain::error::{ActionError, AdapterError};
use std::process::{Command, Output};

pub struct TmuxAdapter;

impl TmuxAdapter {
    pub fn new() -> Self {
        Self
    }
}

impl Default for TmuxAdapter {
    fn default() -> Self {
        Self::new()
    }
}

fn stderr_detail(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).trim().to_string()
}

fn run_action_command(
    args: &[&str],
    error: impl Fn(String) -> ActionError,
) -> Result<Output, ActionError> {
    let output = Command::new("tmux")
        .args(args)
        .output()
        .map_err(|e| error(e.to_string()))?;

    if !output.status.success() {
        return Err(error(stderr_detail(&output)));
    }

    Ok(output)
}

impl TmuxSource for TmuxAdapter {
    fn list_windows(&self) -> Result<Vec<RawWindow>, AdapterError> {
        let fmt = "#{session_name}\t#{window_index}\t#{window_name}\t#{pane_current_path}\t#{@pi_original_path}\t#{@pi_display_name}\t#{window_activity}";
        let output = run_tmux(&["list-windows", "-a", "-F", fmt])?;
        parse_windows(&output)
    }

    fn list_sessions(&self) -> Result<Vec<RawSession>, AdapterError> {
        let fmt = "#{session_name}\t#{session_attached}\t#{session_activity}";
        let output = run_tmux(&["list-sessions", "-F", fmt])?;
        parse_sessions(&output)
    }

    fn current_session(&self) -> Result<String, AdapterError> {
        let output = run_tmux(&["display-message", "-p", "#{session_name}"])?;
        Ok(output)
    }

    fn current_window_index(&self) -> Result<String, AdapterError> {
        let output = run_tmux(&["display-message", "-p", "#{window_index}"])?;
        Ok(output)
    }

    fn has_session(&self, name: &str) -> Result<bool, AdapterError> {
        let output = Command::new("tmux")
            .args(["has-session", "-t", name])
            .output()
            .map_err(|e| AdapterError::TmuxCommand {
                command: format!("has-session -t {name}"),
                detail: e.to_string(),
            })?;
        Ok(output.status.success())
    }

    fn select_window(&self, target: &str) -> Result<(), ActionError> {
        run_action_command(&["select-window", "-t", target], |detail| {
            ActionError::GotoFailed {
                target: target.to_string(),
                detail,
            }
        })?;
        Ok(())
    }

    fn new_session(&self, name: &str, path: &str) -> Result<(), ActionError> {
        run_action_command(&["new-session", "-d", "-s", name, "-c", path], |detail| {
            ActionError::GotoFailed {
                target: name.to_string(),
                detail,
            }
        })?;
        Ok(())
    }

    fn new_window(&self, session: &str, path: &str) -> Result<String, ActionError> {
        let output = run_action_command(
            &[
                "new-window",
                "-P",
                "-F",
                "#{window_index}",
                "-t",
                session,
                "-c",
                path,
            ],
            |detail| ActionError::GotoFailed {
                target: session.to_string(),
                detail,
            },
        )?;
        let window_index = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if window_index.is_empty() {
            return Err(ActionError::GotoFailed {
                target: session.to_string(),
                detail: "tmux new-window returned empty index".to_string(),
            });
        }
        Ok(format!("{session}:{window_index}"))
    }

    fn switch_client(&self, target: &str) -> Result<(), ActionError> {
        run_action_command(&["switch-client", "-t", target], |detail| {
            ActionError::GotoFailed {
                target: target.to_string(),
                detail,
            }
        })?;
        Ok(())
    }

    fn kill_window(&self, target: &str) -> Result<(), ActionError> {
        run_action_command(&["kill-window", "-t", target], |detail| {
            ActionError::KillFailed {
                target: target.to_string(),
                detail,
            }
        })?;
        Ok(())
    }

    fn kill_session(&self, name: &str) -> Result<(), ActionError> {
        run_action_command(&["kill-session", "-t", name], |detail| {
            ActionError::KillFailed {
                target: name.to_string(),
                detail,
            }
        })?;
        Ok(())
    }

    fn set_window_option(
        &self,
        target: &str,
        option: &str,
        value: &str,
    ) -> Result<(), ActionError> {
        run_action_command(
            &["set-option", "-w", "-t", target, option, value],
            |detail| ActionError::GotoFailed {
                target: target.to_string(),
                detail,
            },
        )?;
        Ok(())
    }

    fn capture_pane(&self, target: &str, line_count: usize) -> Result<String, AdapterError> {
        capture_best_effort(target, line_count)
    }

    fn capture_pane_with_size(
        &self,
        target: &str,
        line_count: usize,
        width: Option<u16>,
        height: Option<u16>,
    ) -> Result<String, AdapterError> {
        capture_best_effort_with_size(target, line_count, width, height)
    }
}
