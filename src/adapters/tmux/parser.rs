use crate::adapters::tmux::raw::{RawSession, RawWindow};
use crate::domain::error::AdapterError;

pub fn parse_windows(output: &str) -> Result<Vec<RawWindow>, AdapterError> {
    output
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| {
            let parts: Vec<&str> = line.split('\t').collect();
            if parts.len() != 4 {
                return Err(AdapterError::TmuxParse {
                    input: line.to_string(),
                    detail: format!("expected 4 tab-separated fields, got {}", parts.len()),
                });
            }
            Ok(RawWindow {
                session_name: parts[0].to_string(),
                window_index: parts[1].to_string(),
                window_name: parts[2].to_string(),
                window_path: parts[3].to_string(),
            })
        })
        .collect()
}

pub fn parse_sessions(output: &str) -> Result<Vec<RawSession>, AdapterError> {
    output
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| {
            let parts: Vec<&str> = line.split('\t').collect();
            if parts.len() != 2 {
                return Err(AdapterError::TmuxParse {
                    input: line.to_string(),
                    detail: format!("expected 2 tab-separated fields, got {}", parts.len()),
                });
            }
            Ok(RawSession {
                session_name: parts[0].to_string(),
                attached: parts[1] == "1",
            })
        })
        .collect()
}
