use std::fs::{self, Metadata};
use std::os::unix::fs::PermissionsExt;
use std::path::Path;

use chrono::{DateTime, Local};
use devicons::FileIcon;
use nu_ansi_term::{Color, Style};

use crate::adapters::tmux::TmuxSource;
use crate::domain::entry::Entry;
use crate::domain::path_name::basename_from_path;
use crate::preview::types::DirectoryListingContent;

pub fn generate_directory_preview_content(
    _tmux: &dyn TmuxSource,
    entry: &Entry,
    source: &str,
    max_dir_entries: usize,
) -> Result<DirectoryListingContent, String> {
    if let Err(err) = fs::read_dir(&entry.path) {
        return Err(format!("cannot read directory '{}': {err}", entry.path));
    }

    let dir_name = basename_from_path(&entry.path);
    let listing = read_directory(&entry.path, max_dir_entries);

    Ok(DirectoryListingContent {
        name: dir_name,
        path: entry.path.clone(),
        headline: String::new(),
        entries: listing,
        has_session: false,
        source: source.to_string(),
    })
}

fn colorize(c: char, style: Style) -> String {
    style.paint(c.to_string()).to_string()
}

const TN_BLUE: Color = Color::Fixed(39); // ~#7aa2f7
const TN_CYAN: Color = Color::Fixed(117); // ~#89ddff
const TN_YELLOW: Color = Color::Fixed(215); // ~#e0af68
const TN_RED: Color = Color::Fixed(210); // ~#f7768e
const TN_GREEN: Color = Color::Fixed(149); // ~#9ece6a
const TN_DIM: Color = Color::Fixed(244);

fn format_permissions_colored(meta: &Metadata) -> String {
    let mode = meta.permissions().mode();

    let file_type = if meta.is_dir() {
        colorize('d', TN_BLUE.bold())
    } else if meta.file_type().is_symlink() {
        colorize('l', TN_CYAN.bold())
    } else {
        colorize('-', TN_YELLOW.normal())
    };

    let mut perms = file_type;
    let bits = [
        (0o400, 'r', TN_YELLOW),
        (0o200, 'w', TN_RED),
        (0o100, 'x', TN_GREEN),
        (0o040, 'r', TN_YELLOW),
        (0o020, 'w', TN_RED),
        (0o010, 'x', TN_GREEN),
        (0o004, 'r', TN_YELLOW),
        (0o002, 'w', TN_RED),
        (0o001, 'x', TN_GREEN),
    ];

    for (bit, ch, color) in bits.iter() {
        let ch_str = if mode & bit != 0 {
            colorize(*ch, color.normal())
        } else {
            colorize('-', TN_DIM.normal())
        };
        perms.push_str(&ch_str);
    }

    perms
}

fn format_size_colored(meta: &Metadata) -> String {
    if meta.is_dir() {
        return TN_DIM.paint("-").to_string();
    }

    let size = meta.len();
    let (size_str, style): (String, Style) = if size >= 1024 * 1024 * 1024 {
        let gb = size as f64 / (1024.0 * 1024.0 * 1024.0);
        (format!("{:.1}G", gb), TN_RED.normal())
    } else if size >= 1024 * 1024 {
        let mb = size as f64 / (1024.0 * 1024.0);
        (format!("{:.1}M", mb), TN_YELLOW.normal())
    } else if size >= 1024 {
        let kb = size as f64 / 1024.0;
        if kb >= 100.0 {
            (format!("{:.0}k", kb), TN_GREEN.bold())
        } else {
            (format!("{:.1}k", kb), TN_GREEN.normal())
        }
    } else {
        (format!("{}", size), Color::Fixed(242).normal())
    };

    style.paint(size_str).to_string()
}

fn format_date_colored(meta: &Metadata) -> String {
    let modified = match meta.modified() {
        Ok(m) => m,
        Err(_) => return TN_DIM.paint("--- -- --:--").to_string(),
    };

    let local: DateTime<Local> = DateTime::from(modified);
    TN_BLUE
        .paint(local.format("%b %d %H:%M").to_string())
        .to_string()
}

fn hex_to_color(hex: &str) -> Option<Color> {
    let hex = hex.trim_start_matches('#');
    if hex.len() != 6 {
        return None;
    }
    let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
    let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
    let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
    Some(Color::Rgb(r, g, b))
}

fn get_file_icon_and_color(path: &Path) -> (char, Color) {
    if path.is_dir() {
        return ('\u{e5ff}', TN_BLUE);
    }

    let file_icon = FileIcon::from(path);
    let color = hex_to_color(file_icon.color).unwrap_or(Color::White);
    (file_icon.icon, color)
}

fn pad_to_width(s: &str, width: usize) -> String {
    let visible_len = strip_ansi(s).len();
    if visible_len >= width {
        s.to_string()
    } else {
        let padding = " ".repeat(width - visible_len);
        format!("{}{}", padding, s)
    }
}

pub fn read_directory(path: &str, max_entries: usize) -> Vec<String> {
    let dir = match fs::read_dir(path) {
        Ok(d) => d,
        Err(_) => return vec!["(cannot read directory)".to_string()],
    };

    struct EntryInfo {
        is_dir: bool,
        name: String,
        perms: String,
        size: String,
        modified: String,
        icon: char,
        icon_color: Color,
    }

    let mut entries: Vec<EntryInfo> = dir
        .filter_map(|entry| {
            let entry = entry.ok()?;
            let full_path = entry.path();
            let name = entry.file_name().to_string_lossy().to_string();

            if name.starts_with('.') {
                return None;
            }

            let meta = fs::symlink_metadata(&full_path).ok()?;
            let (icon, icon_color) = get_file_icon_and_color(&full_path);

            Some(EntryInfo {
                is_dir: meta.is_dir(),
                name,
                perms: format_permissions_colored(&meta),
                size: format_size_colored(&meta),
                modified: format_date_colored(&meta),
                icon,
                icon_color,
            })
        })
        .collect();

    entries.sort_by(|a, b| b.is_dir.cmp(&a.is_dir).then_with(|| a.name.cmp(&b.name)));
    entries.truncate(max_entries);

    let max_size_width = entries
        .iter()
        .map(|e| strip_ansi(&e.size).len())
        .max()
        .unwrap_or(4);

    entries
        .into_iter()
        .map(|e| {
            let style = if e.is_dir {
                e.icon_color.bold()
            } else {
                e.icon_color.normal()
            };

            let colored_icon = style.paint(e.icon.to_string()).to_string();
            let colored_name = style.paint(&e.name).to_string();

            let size_padded = pad_to_width(&e.size, max_size_width);

            format!(
                "{} {}  {}  {} {}",
                e.perms, size_padded, e.modified, colored_icon, colored_name,
            )
        })
        .collect()
}

fn strip_ansi(input: &str) -> String {
    let mut result = String::with_capacity(input.len());
    let mut chars = input.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '\x1b' && chars.peek() == Some(&'[') {
            chars.next();
            for ch in chars.by_ref() {
                if ch == 'm' {
                    break;
                }
            }
        } else {
            result.push(c);
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strip_ansi_removes_color_codes() {
        assert_eq!(strip_ansi("\x1b[31mtest\x1b[0m"), "test");
        assert_eq!(strip_ansi("\x1b[1;32mhello\x1b[0m"), "hello");
    }

    #[test]
    fn pad_to_width_works_with_ansi() {
        let colored = "\x1b[31mred\x1b[0m";
        let padded = pad_to_width(colored, 10);
        assert_eq!(strip_ansi(&padded).len(), 10);
    }
}
