pub fn strip_ansi(input: &str) -> String {
    let mut result = String::with_capacity(input.len());
    let mut chars = input.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '\x1b' {
            match chars.peek() {
                Some('{') => {
                    chars.next();
                    skip_until_closing_brace(&mut chars);
                }
                Some('[') => {
                    chars.next();
                    skip_csi_sequence(&mut chars);
                }
                Some(']') => {
                    chars.next();
                    skip_osc_sequence(&mut chars);
                }
                _ => {}
            }
        } else {
            result.push(c);
        }
    }

    result
}

fn skip_until_closing_brace(chars: &mut std::iter::Peekable<std::str::Chars<'_>>) {
    for c in chars.by_ref() {
        if c == '}' {
            break;
        }
    }
}

fn skip_csi_sequence(chars: &mut std::iter::Peekable<std::str::Chars<'_>>) {
    for c in chars.by_ref() {
        if c == '\x1b' {
            break;
        }
        if ('\x40'..='\x7e').contains(&c) {
            break;
        }
    }
}

fn skip_osc_sequence(chars: &mut std::iter::Peekable<std::str::Chars<'_>>) {
    for c in chars.by_ref() {
        if c == '\x07' {
            break;
        }
    }
}

pub fn strip_ansi_lines(input: &str, max_lines: usize) -> Vec<String> {
    let stripped = strip_ansi(input);
    stripped
        .lines()
        .filter(|line| !line.trim().is_empty())
        .take(max_lines)
        .map(|l| l.to_string())
        .collect()
}

pub fn strip_ansi_lines_tail(input: &str, max_lines: usize) -> Vec<String> {
    if max_lines == 0 {
        return vec![];
    }

    let stripped = strip_ansi(input);
    let lines: Vec<String> = stripped.lines().map(ToString::to_string).collect();
    if lines.len() <= max_lines {
        return lines;
    }
    lines[lines.len() - max_lines..].to_vec()
}

pub fn truncate_lines(lines: &[String], max_lines: usize) -> Vec<String> {
    lines.iter().take(max_lines).cloned().collect()
}

pub fn truncate_line_width(lines: &mut [String], max_width: usize) {
    for line in lines.iter_mut() {
        if line.chars().count() > max_width {
            let truncated: String = line.chars().take(max_width - 1).collect();
            *line = format!("{truncated}…");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strip_ansi_color_code() {
        assert_eq!(strip_ansi("\x1b[31mhello\x1b[0m"), "hello");
    }

    #[test]
    fn strip_ansi_rgb_code() {
        assert_eq!(strip_ansi("\x1b[38;5;123mcolored\x1b[0m"), "colored");
    }

    #[test]
    fn strip_ansi_bold() {
        assert_eq!(strip_ansi("\x1b[1mbold\x1b[22m"), "bold");
    }

    #[test]
    fn strip_ansi_no_ansi() {
        assert_eq!(strip_ansi("plain text"), "plain text");
    }

    #[test]
    fn strip_ansi_empty_string() {
        assert_eq!(strip_ansi(""), "");
    }

    #[test]
    fn strip_ansi_multiple_codes() {
        let input = "\x1b[31mred\x1b[32mgreen\x1b[0m plain";
        assert_eq!(strip_ansi(input), "redgreen plain");
    }

    #[test]
    fn strip_ansi_implicit_reset() {
        let input = "\x1b[31mred\x1b[m reset";
        assert_eq!(strip_ansi(input), "red reset");
    }

    #[test]
    fn strip_ansi_incomplete_at_end() {
        assert_eq!(strip_ansi("\x1b[31m"), "");
    }

    #[test]
    fn strip_ansi_lines_filters_empty() {
        let input = "line1\n\nline2\n\nline3";
        let result = strip_ansi_lines(input, 10);
        assert_eq!(result, vec!["line1", "line2", "line3"]);
    }

    #[test]
    fn strip_ansi_lines_respects_max() {
        let input = "a\nb\nc\nd\ne";
        let result = strip_ansi_lines(input, 3);
        assert_eq!(result.len(), 3);
        assert_eq!(result[0], "a");
        assert_eq!(result[2], "c");
    }

    #[test]
    fn strip_ansi_lines_strips_ansi() {
        let input = "\x1b[31mred\x1b[0m\nplain\n\x1b[1mbold\x1b[22m";
        let result = strip_ansi_lines(input, 10);
        assert_eq!(result, vec!["red", "plain", "bold"]);
    }

    #[test]
    fn strip_ansi_lines_tail_returns_latest_lines() {
        let input = "a\nb\nc\nd\ne";
        let result = strip_ansi_lines_tail(input, 2);
        assert_eq!(result, vec!["d", "e"]);
    }

    #[test]
    fn truncate_lines_within_limit() {
        let lines = vec!["a".into(), "b".into(), "c".into(), "d".into()];
        assert_eq!(truncate_lines(&lines, 2), vec!["a", "b"]);
    }

    #[test]
    fn truncate_lines_empty_input() {
        assert!(truncate_lines(&[], 5).is_empty());
    }

    #[test]
    fn truncate_lines_under_limit() {
        let lines = vec!["a".into()];
        assert_eq!(truncate_lines(&lines, 5), vec!["a"]);
    }

    #[test]
    fn truncate_line_width_short_line_unchanged() {
        let mut lines = vec!["hello".into()];
        truncate_line_width(&mut lines, 80);
        assert_eq!(lines, vec!["hello"]);
    }

    #[test]
    fn truncate_line_width_exact_limit_unchanged() {
        let line = "a".repeat(80);
        let mut lines = vec![line.clone()];
        truncate_line_width(&mut lines, 80);
        assert_eq!(lines, vec![line]);
    }

    #[test]
    fn truncate_line_width_exceeds_limit() {
        let line = "a".repeat(100);
        let mut lines = vec![line.clone()];
        truncate_line_width(&mut lines, 80);
        assert_eq!(lines[0].chars().count(), 80);
        assert!(lines[0].ends_with('…'));
    }

    #[test]
    fn truncate_line_width_empty_input() {
        let mut lines: Vec<String> = vec![];
        truncate_line_width(&mut lines, 80);
        assert!(lines.is_empty());
    }

    #[test]
    fn truncate_line_width_multiple_lines() {
        let mut lines = vec!["short".into(), "a".repeat(90), "ok".into()];
        truncate_line_width(&mut lines, 80);
        assert_eq!(lines[0], "short");
        assert_eq!(lines[0].chars().count(), 5);
        assert_eq!(lines[1].chars().count(), 80);
        assert_eq!(lines[2], "ok");
    }
}
