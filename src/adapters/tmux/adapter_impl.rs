use crate::adapters::tmux::capture::{capture_best_effort, capture_best_effort_with_size};
use crate::adapters::tmux::command::run_tmux;
use crate::adapters::tmux::parser::{parse_sessions, parse_windows};
use crate::adapters::tmux::raw::{RawSession, RawWindow};
use crate::adapters::tmux::TmuxSource;
use crate::domain::error::{ActionError, AdapterError};
use std::process::Command;

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

impl TmuxSource for TmuxAdapter {
    fn list_windows(&self) -> Result<Vec<RawWindow>, AdapterError> {
        let fmt = "#{session_name}\t#{window_index}\t#{window_name}\t#{pane_current_path}";
        let output = run_tmux(&["list-windows", "-a", "-F", fmt])?;
        parse_windows(&output)
    }

    fn list_sessions(&self) -> Result<Vec<RawSession>, AdapterError> {
        let fmt = "#{session_name}\t#{session_attached}";
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
        let output = Command::new("tmux")
            .args(["select-window", "-t", target])
            .output()
            .map_err(|e| ActionError::GotoFailed {
                target: target.to_string(),
                detail: e.to_string(),
            })?;
        if !output.status.success() {
            return Err(ActionError::GotoFailed {
                target: target.to_string(),
                detail: String::from_utf8_lossy(&output.stderr).trim().to_string(),
            });
        }
        Ok(())
    }

    fn new_session(&self, name: &str, path: &str) -> Result<(), ActionError> {
        let output = Command::new("tmux")
            .args(["new-session", "-d", "-s", name, "-c", path])
            .output()
            .map_err(|e| ActionError::GotoFailed {
                target: name.to_string(),
                detail: e.to_string(),
            })?;
        if !output.status.success() {
            return Err(ActionError::GotoFailed {
                target: name.to_string(),
                detail: String::from_utf8_lossy(&output.stderr).trim().to_string(),
            });
        }
        Ok(())
    }

    fn new_window(&self, session: &str, path: &str) -> Result<String, ActionError> {
        let output = Command::new("tmux")
            .args([
                "new-window",
                "-P",
                "-F",
                "#{window_index}",
                "-t",
                session,
                "-c",
                path,
            ])
            .output()
            .map_err(|e| ActionError::GotoFailed {
                target: session.to_string(),
                detail: e.to_string(),
            })?;
        if !output.status.success() {
            return Err(ActionError::GotoFailed {
                target: session.to_string(),
                detail: String::from_utf8_lossy(&output.stderr).trim().to_string(),
            });
        }
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
        let output = Command::new("tmux")
            .args(["switch-client", "-t", target])
            .output()
            .map_err(|e| ActionError::GotoFailed {
                target: target.to_string(),
                detail: e.to_string(),
            })?;
        if !output.status.success() {
            return Err(ActionError::GotoFailed {
                target: target.to_string(),
                detail: String::from_utf8_lossy(&output.stderr).trim().to_string(),
            });
        }
        Ok(())
    }

    fn kill_window(&self, target: &str) -> Result<(), ActionError> {
        let output = Command::new("tmux")
            .args(["kill-window", "-t", target])
            .output()
            .map_err(|e| ActionError::KillFailed {
                target: target.to_string(),
                detail: e.to_string(),
            })?;
        if !output.status.success() {
            return Err(ActionError::KillFailed {
                target: target.to_string(),
                detail: String::from_utf8_lossy(&output.stderr).trim().to_string(),
            });
        }
        Ok(())
    }

    fn kill_session(&self, name: &str) -> Result<(), ActionError> {
        let output = Command::new("tmux")
            .args(["kill-session", "-t", name])
            .output()
            .map_err(|e| ActionError::KillFailed {
                target: name.to_string(),
                detail: e.to_string(),
            })?;
        if !output.status.success() {
            return Err(ActionError::KillFailed {
                target: name.to_string(),
                detail: String::from_utf8_lossy(&output.stderr).trim().to_string(),
            });
        }
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
