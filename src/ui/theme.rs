// Tokyo Night palette (dark theme - night variant)
pub mod colors {
    use ratatui::style::Color;

    // Background colors
    pub const BG_PRIMARY: Color = Color::Rgb(26, 27, 38); // bg #1a1b26
    pub const BG_SECONDARY: Color = Color::Rgb(22, 22, 30); // bg_dark #16161e
    pub const BG_TERTIARY: Color = Color::Rgb(41, 46, 66); // bg_highlight #292e42

    // Text colors
    pub const TEXT_PRIMARY: Color = Color::Rgb(192, 202, 245); // fg #c0caf5
    pub const TEXT_SECONDARY: Color = Color::Rgb(169, 177, 214); // fg_dark #a9b1d6
    pub const TEXT_MUTED: Color = Color::Rgb(86, 95, 137); // comment #565f89
    pub const TEXT_DISABLED: Color = Color::Rgb(84, 92, 126); // dark3 #545c7e

    // Accent colors
    pub const ACCENT_CYAN: Color = Color::Rgb(137, 221, 255); // blue5 #89ddff
    pub const ACCENT_BLUE: Color = Color::Rgb(122, 162, 247); // blue #7aa2f7
    pub const ACCENT_GREEN: Color = Color::Rgb(158, 206, 106); // green #9ece6a
    pub const ACCENT_YELLOW: Color = Color::Rgb(224, 175, 104); // yellow #e0af68
    pub const ACCENT_RED: Color = Color::Rgb(247, 118, 142); // red #f7768e

    // Border colors
    pub const BORDER_DEFAULT: Color = Color::Rgb(59, 66, 97); // fg_gutter #3b4261
    pub const BORDER_FOCUSED: Color = Color::Rgb(42, 195, 222); // blue1 #2ac3de
    pub const BORDER_ACTIVE: Color = Color::Rgb(122, 162, 247); // blue #7aa2f7
}

// Semantic styles
pub mod styles {
    use super::colors::*;
    use ratatui::style::{Modifier, Style};

    pub fn block_default() -> Style {
        Style::default().fg(BORDER_DEFAULT)
    }

    pub fn block_focused() -> Style {
        Style::default().fg(BORDER_FOCUSED)
    }

    pub fn text_normal() -> Style {
        Style::default().fg(TEXT_SECONDARY)
    }

    pub fn text_highlighted() -> Style {
        Style::default()
            .fg(TEXT_PRIMARY)
            .add_modifier(Modifier::BOLD)
    }

    pub fn text_muted() -> Style {
        Style::default().fg(TEXT_MUTED)
    }

    pub fn text_current() -> Style {
        Style::default()
            .fg(ACCENT_YELLOW)
            .add_modifier(Modifier::BOLD)
    }

    pub fn list_item_normal() -> Style {
        Style::default().fg(TEXT_SECONDARY)
    }

    pub fn list_item_selected() -> Style {
        Style::default()
            .fg(TEXT_PRIMARY)
            .add_modifier(Modifier::BOLD)
    }

    pub fn list_item_current() -> Style {
        Style::default().fg(ACCENT_YELLOW)
    }

    pub fn list_header() -> Style {
        Style::default()
            .fg(ACCENT_CYAN)
            .add_modifier(Modifier::BOLD)
    }

    pub fn search_container() -> Style {
        Style::default().bg(BG_SECONDARY).fg(BORDER_FOCUSED)
    }

    pub fn search_text() -> Style {
        Style::default()
            .fg(TEXT_PRIMARY)
            .add_modifier(Modifier::BOLD)
    }

    pub fn search_placeholder() -> Style {
        Style::default().fg(TEXT_MUTED)
    }

    pub fn help_key() -> Style {
        Style::default()
            .fg(ACCENT_CYAN)
            .add_modifier(Modifier::BOLD)
    }

    pub fn help_description() -> Style {
        Style::default().fg(TEXT_MUTED)
    }

    pub fn help_separator() -> Style {
        Style::default().fg(BORDER_DEFAULT)
    }

    pub fn help_version() -> Style {
        Style::default()
            .fg(ACCENT_GREEN)
            .add_modifier(Modifier::BOLD)
    }

    pub fn preview_header() -> Style {
        Style::default()
            .fg(ACCENT_CYAN)
            .add_modifier(Modifier::BOLD)
    }

    pub fn preview_label() -> Style {
        Style::default().fg(TEXT_MUTED)
    }

    pub fn preview_value() -> Style {
        Style::default().fg(TEXT_SECONDARY)
    }

    pub fn preview_status_active() -> Style {
        Style::default().fg(ACCENT_GREEN)
    }

    pub fn preview_status_inactive() -> Style {
        Style::default().fg(TEXT_MUTED)
    }
}

pub mod icons {
    pub const SESSION_COLLAPSED: &str = "";
    pub const SESSION_EXPANDED: &str = "";
    pub const WINDOW: &str = "";
    pub const WINDOW_CURRENT: &str = "▸";
    pub const FOLDER: &str = "";
    pub const FOLDER_OPEN: &str = "";
    pub const SEARCH: &str = "";
    pub const SEARCH_CLEAR: &str = "";
    pub const CURSOR: &str = "▏";
    pub const CHECK: &str = "";
    pub const CROSS: &str = "";
    pub const LOADING: &str = "";
    pub const ARROW_RIGHT: &str = "";
    pub const ARROW_SUB: &str = "└─";
    pub const PREVIEW: &str = "";
    pub const TERMINAL: &str = "";
    pub const DIRECTORY: &str = "";
    pub const FILE: &str = "";
}

// Layout constants
pub mod layout {
    pub const SEARCH_HEIGHT: u16 = 3;
    pub const HELP_HEIGHT: u16 = 1;
    pub const LIST_WIDTH_PERCENT: u16 = 20;
    pub const PREVIEW_WIDTH_PERCENT: u16 = 80;
    pub const HIGHLIGHT_SYMBOL: &str = "▶ ";
}
