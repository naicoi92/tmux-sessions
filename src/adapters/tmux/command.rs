use crate::domain::error::AdapterError;
use std::process::{Command, Output};

pub fn run_tmux(args: &[&str]) -> Result<String, AdapterError> {
    let command = args.join(" ");
    let output =
        Command::new("tmux")
            .args(args)
            .output()
            .map_err(|e| AdapterError::TmuxCommand {
                command: command.clone(),
                detail: e.to_string(),
            })?;
    handle_tmux_output(&command, output)
}

pub fn handle_tmux_output(command: &str, output: Output) -> Result<String, AdapterError> {
    if !output.status.success() {
        return Err(AdapterError::TmuxCommand {
            command: command.to_string(),
            detail: String::from_utf8_lossy(&output.stderr).trim().to_string(),
        });
    }

    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

pub fn handle_tmux_capture_output(command: &str, output: Output) -> Result<String, AdapterError> {
    if !output.status.success() {
        return Err(AdapterError::TmuxCommand {
            command: command.to_string(),
            detail: String::from_utf8_lossy(&output.stderr).trim().to_string(),
        });
    }

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

pub fn run_capture_pane(args: &[String]) -> Result<String, AdapterError> {
    let command = args.join(" ");
    let output =
        Command::new("tmux")
            .args(args)
            .output()
            .map_err(|e| AdapterError::TmuxCommand {
                command: command.clone(),
                detail: e.to_string(),
            })?;
    handle_tmux_capture_output(&command, output)
}
