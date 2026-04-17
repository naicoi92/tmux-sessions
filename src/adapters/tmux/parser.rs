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
            if parts.len() != 3 {
                return Err(AdapterError::TmuxParse {
                    input: line.to_string(),
                    detail: format!("expected 3 tab-separated fields, got {}", parts.len()),
                });
            }
            let session_activity = if parts[2].is_empty() {
                None
            } else {
                parts[2].parse::<i64>().ok()
            };
            Ok(RawSession {
                session_name: parts[0].to_string(),
                attached: parts[1] == "1",
                session_activity,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_sessions_accepts_session_activity_field() {
        let input = "s1\t1\t1714000000\ns2\t0\t1714000100";
        let result = parse_sessions(input).unwrap();

        assert_eq!(result.len(), 2);
        assert_eq!(result[0].session_name, "s1");
        assert!(result[0].attached);
        assert_eq!(result[0].session_activity, Some(1714000000));
        assert_eq!(result[1].session_activity, Some(1714000100));
    }

    #[test]
    fn parse_sessions_invalid_or_empty_activity_becomes_none() {
        let input = "s1\t1\t\ns2\t0\tabc\ns3\t1\t0";
        let result = parse_sessions(input).unwrap();

        assert_eq!(result.len(), 3);
        assert_eq!(result[0].session_activity, None);
        assert_eq!(result[1].session_activity, None);
        assert_eq!(result[2].session_activity, Some(0));
    }

    #[test]
    fn parse_sessions_requires_three_fields() {
        let input = "s1\t1";
        let result = parse_sessions(input);
        assert!(result.is_err());
    }

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

    #[test]
    fn parse_windows_negative_activity_accepted() {
        let input = "s1\t0\tmain\t/home/user\t-1";
        let result = parse_windows(input).unwrap();
        assert_eq!(result[0].window_activity, Some(-1));
    }

    #[test]
    fn parse_windows_zero_activity_accepted() {
        let input = "s1\t0\tmain\t/home/user\t0";
        let result = parse_windows(input).unwrap();
        assert_eq!(result[0].window_activity, Some(0));
    }

    #[test]
    fn parse_windows_large_timestamp_accepted() {
        let input = "s1\t0\tmain\t/home/user\t9999999999999";
        let result = parse_windows(input).unwrap();
        assert_eq!(result[0].window_activity, Some(9999999999999));
    }

    #[test]
    fn parse_windows_all_empty_activity() {
        let input = "s1\t0\tmain\t/home/user\t\ns2\t1\tother\t/tmp\t";
        let result = parse_windows(input).unwrap();
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].window_activity, None);
        assert_eq!(result[1].window_activity, None);
    }

    #[test]
    fn parse_windows_single_line_valid() {
        let input = "s1\t0\tmain\t/home/user\t1714000000";
        let result = parse_windows(input).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].window_activity, Some(1714000000));
    }

    #[test]
    fn parse_windows_whitespace_only_activity_becomes_none() {
        let input = "s1\t0\tmain\t/home/user\t   ";
        let result = parse_windows(input).unwrap();
        assert_eq!(result[0].window_activity, None);
    }

    #[test]
    fn parse_windows_float_activity_becomes_none() {
        let input = "s1\t0\tmain\t/home/user\t1714000.5";
        let result = parse_windows(input).unwrap();
        assert_eq!(result[0].window_activity, None);
    }

    #[test]
    fn parse_windows_missing_field_errors() {
        let input = "s1\t0\tmain\t/home/user";
        let result = parse_windows(input);
        assert!(result.is_err());
    }

    #[test]
    fn parse_windows_too_many_fields_errors() {
        let input = "s1\t0\tmain\t/home/user\t1714000000\textra";
        let result = parse_windows(input);
        assert!(result.is_err());
    }

    #[test]
    fn parse_windows_empty_input() {
        let result = parse_windows("").unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn parse_windows_whitespace_only_lines_skipped() {
        let input = "s1\t0\tmain\t/home/user\t100\n   \ns2\t1\tother\t/tmp\t200";
        let result = parse_windows(input).unwrap();
        assert_eq!(result.len(), 2);
    }
}
