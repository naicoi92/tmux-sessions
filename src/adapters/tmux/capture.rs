use crate::adapters::tmux::command::run_capture_pane;
use crate::domain::error::AdapterError;
use crate::preview::ansi::strip_ansi;

pub fn capture_pane_args(target: &str, line_count: usize, alternate_screen: bool) -> Vec<String> {
    let mut args = vec![
        "capture-pane".to_string(),
        "-t".to_string(),
        target.to_string(),
        "-p".to_string(),
        "-e".to_string(),
    ];
    if alternate_screen {
        args.push("-a".to_string());
    }
    args.extend(["-S".to_string(), format!("-{line_count}"), "-N".to_string()]);
    args
}

pub fn select_capture_content(
    primary: Result<String, AdapterError>,
    alternate: Result<String, AdapterError>,
) -> Result<String, AdapterError> {
    match (primary, alternate) {
        (Ok(primary_content), Ok(alternate_content)) => {
            let primary_score = visible_content_score(&primary_content);
            let alternate_score = visible_content_score(&alternate_content);
            let primary_lines = visible_line_count(&primary_content);
            let alternate_lines = visible_line_count(&alternate_content);

            if (primary_score == 0 && alternate_score > 0)
                || should_prefer_alternate(
                    primary_score,
                    primary_lines,
                    alternate_score,
                    alternate_lines,
                )
            {
                Ok(alternate_content)
            } else if !primary_content.is_empty() {
                Ok(primary_content)
            } else if !alternate_content.is_empty() {
                Ok(alternate_content)
            } else {
                Ok(primary_content)
            }
        }
        (Ok(primary_content), Err(_)) => Ok(primary_content),
        (Err(_), Ok(alternate_content)) => Ok(alternate_content),
        (Err(primary_err), Err(_)) => Err(primary_err),
    }
}

pub fn visible_content_score(content: &str) -> usize {
    strip_ansi(content)
        .lines()
        .map(|line| line.trim())
        .filter(|line| !line.is_empty())
        .map(str::len)
        .sum()
}

pub fn visible_line_count(content: &str) -> usize {
    strip_ansi(content)
        .lines()
        .map(|line| line.trim())
        .filter(|line| !line.is_empty())
        .count()
}

pub fn should_prefer_alternate(
    primary_score: usize,
    primary_lines: usize,
    alternate_score: usize,
    alternate_lines: usize,
) -> bool {
    primary_score <= 20
        && primary_lines <= 2
        && alternate_lines >= 2
        && alternate_score >= primary_score.saturating_add(15)
}

pub fn capture_best_effort(target: &str, line_count: usize) -> Result<String, AdapterError> {
    let primary_args = capture_pane_args(target, line_count, false);
    let primary_result = run_capture_pane(&primary_args);

    let should_probe_alternate = match &primary_result {
        Ok(primary_content) => {
            let primary_score = visible_content_score(primary_content);
            let primary_lines = visible_line_count(primary_content);
            primary_score == 0 || (primary_score <= 20 && primary_lines <= 2)
        }
        Err(_) => true,
    };

    if should_probe_alternate {
        let alternate_args = capture_pane_args(target, line_count, true);
        let alternate_result = run_capture_pane(&alternate_args);
        select_capture_content(primary_result, alternate_result)
    } else {
        primary_result
    }
}

pub fn capture_pane_args_with_size(
    target: &str,
    line_count: usize,
    _width: Option<u16>,
    height: Option<u16>,
    alternate_screen: bool,
) -> Vec<String> {
    let effective_line_count = height.map(|h| h as usize).unwrap_or(line_count);
    let mut args = vec![
        "capture-pane".to_string(),
        "-t".to_string(),
        target.to_string(),
        "-p".to_string(),
        "-e".to_string(),
    ];
    if alternate_screen {
        args.push("-a".to_string());
    }
    args.extend([
        "-S".to_string(),
        format!("-{effective_line_count}"),
        "-N".to_string(),
    ]);
    args
}

pub fn capture_best_effort_with_size(
    target: &str,
    line_count: usize,
    width: Option<u16>,
    height: Option<u16>,
) -> Result<String, AdapterError> {
    let primary_args = capture_pane_args_with_size(target, line_count, width, height, false);
    let primary_result = run_capture_pane(&primary_args);

    let should_probe_alternate = match &primary_result {
        Ok(primary_content) => {
            let primary_score = visible_content_score(primary_content);
            let primary_lines = visible_line_count(primary_content);
            primary_score == 0 || (primary_score <= 20 && primary_lines <= 2)
        }
        Err(_) => true,
    };

    if should_probe_alternate {
        let alternate_args = capture_pane_args_with_size(target, line_count, width, height, true);
        let alternate_result = run_capture_pane(&alternate_args);
        select_capture_content(primary_result, alternate_result)
    } else {
        primary_result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn capture_pane_args_with_size_includes_line_window_contract() {
        let args = capture_pane_args_with_size("test:0", 12, Some(100), Some(20), false);
        // Verify args contains "-20" for line count
        assert!(args.contains(&"-20".to_string()));
    }

    #[test]
    fn capture_pane_args_with_size_uses_height_when_provided() {
        // When height is provided, it should override the default line_count
        let args = capture_pane_args_with_size("test:0", 100, Some(80), Some(25), false);
        assert!(args.contains(&"-25".to_string()));
    }

    #[test]
    fn capture_pane_args_with_size_uses_line_count_when_no_height() {
        // When height is None, fall back to line_count
        let args = capture_pane_args_with_size("test:0", 50, Some(80), None, false);
        assert!(args.contains(&"-50".to_string()));
    }
}
