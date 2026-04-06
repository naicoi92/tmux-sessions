use crate::adapters::tmux::TmuxSource;
use crate::domain::entry::Entry;
use crate::preview::types::TmuxScreenContent;

pub fn build_tmux_screen_content(
    entry: &Entry,
    windows: Vec<String>,
    screen_lines: Vec<String>,
) -> TmuxScreenContent {
    TmuxScreenContent {
        session_name: entry.session_name.clone().unwrap_or_default(),
        path: entry.path.clone(),
        target: entry.target.clone(),
        windows,
        screen_lines,
        is_fallback: false,
    }
}

pub fn list_session_windows(tmux: &dyn TmuxSource, entry: &Entry) -> Vec<String> {
    let session = match &entry.session_name {
        Some(s) => s.clone(),
        None => return vec![],
    };

    match tmux.list_windows() {
        Ok(windows) => windows
            .iter()
            .filter(|w| w.session_name == session)
            .map(|w| {
                let marker = "  ●";
                format!(
                    "{marker} 🪟 [{index}]: {name}",
                    index = w.window_index,
                    name = w.window_name
                )
            })
            .collect(),
        Err(_) => vec!["(could not list windows)".to_string()],
    }
}
