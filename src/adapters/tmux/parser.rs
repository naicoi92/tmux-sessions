use crate::adapters::tmux::raw::{RawSession, RawWindow};
use crate::domain::error::AdapterError;

pub fn parse_windows(output: &str) -> Result<Vec<RawWindow>, AdapterError> {
    output
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| {
            let parts: Vec<&str> = line.split('\t').collect();
            if parts.len() != 5 {
                return Err(AdapterError::TmuxParse {
                    input: line.to_string(),
                    detail: format!("expected 5 tab-separated fields, got {}", parts.len()),
                });
            }
            let window_activity = if parts[4].is_empty() {
                None
            } else {
                parts[4].parse::<i64>().ok()
            };
            Ok(RawWindow {
                session_name: parts[0].to_string(),
                window_index: parts[1].to_string(),
                window_name: parts[2].to_string(),
                window_path: parts[3].to_string(),
                window_activity,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_windows_accepts_window_activity_field() {
        let input = "s1\t0\tmain\t/home/user\t1714000000\ns1\t1\tedit\t/home/user\t1714000100";
        let result = parse_windows(input).unwrap();
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].session_name, "s1");
        assert_eq!(result[0].window_index, "0");
        assert_eq!(result[0].window_activity, Some(1714000000));
        assert_eq!(result[1].window_activity, Some(1714000100));
    }

    #[test]
    fn parse_windows_invalid_activity_becomes_none() {
        // Mix: empty, valid, malformed
        let input = "s1\t0\tmain\t/home/user\t\ns1\t1\tedit\t/home/user\t1714000100\ns1\t2\tbad\t/home/user\tabc";
        let result = parse_windows(input).unwrap();
        assert_eq!(result.len(), 3);
        assert_eq!(result[0].window_activity, None);
        assert_eq!(result[1].window_activity, Some(1714000100));
        assert_eq!(result[2].window_activity, None);
    }
}
