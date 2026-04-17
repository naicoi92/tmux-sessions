use std::sync::Arc;

use crate::adapters::tmux::TmuxSource;
use crate::domain::entry::Entry;
use crate::preview::types::PreviewState;
use crate::preview::{assembly, directory, pane_capture};

const MAX_PANE_LINES: usize = 200;
const MAX_DIR_ENTRIES: usize = 15;

type TmuxFactory = dyn Fn() -> Box<dyn TmuxSource + Send> + Send + Sync;

pub struct PreviewGenerator {
    tmux: Box<dyn TmuxSource + Send>,
    factory: Arc<TmuxFactory>,
}

impl PreviewGenerator {
    pub fn new(tmux: Box<dyn TmuxSource + Send>) -> Self {
        Self::with_factory(tmux, || Box::new(crate::adapters::tmux::TmuxAdapter::new()))
    }

    pub fn with_factory<F>(tmux: Box<dyn TmuxSource + Send>, factory: F) -> Self
    where
        F: Fn() -> Box<dyn TmuxSource + Send> + Send + Sync + 'static,
    {
        Self {
            tmux,
            factory: Arc::new(factory),
        }
    }

    #[cfg(test)]
    fn with_factory_for_test<F>(tmux: Box<dyn TmuxSource + Send>, factory: F) -> Self
    where
        F: Fn() -> Box<dyn TmuxSource + Send> + Send + Sync + 'static,
    {
        Self {
            tmux,
            factory: Arc::new(factory),
        }
    }

    pub fn clone_for_thread(&self) -> Self {
        Self {
            tmux: (self.factory)(),
            factory: Arc::clone(&self.factory),
        }
    }

    pub fn generate(
        &self,
        entry: &Entry,
        dimensions: Option<(u16, u16)>,
    ) -> Result<PreviewState, String> {
        match entry.entry_type {
            crate::domain::entry::EntryType::Window => {
                self.generate_window_preview(entry, dimensions)
            }
            crate::domain::entry::EntryType::Zoxide => self.generate_directory_preview(entry),
        }
    }

    fn generate_window_preview(
        &self,
        entry: &Entry,
        dimensions: Option<(u16, u16)>,
    ) -> Result<PreviewState, String> {
        let pane_content = match pane_capture::capture_pane_lines(
            &*self.tmux,
            entry,
            dimensions,
            MAX_PANE_LINES,
        ) {
            Ok(lines) => lines,
            Err(_) => {
                return self
                    .generate_directory_preview_internal(entry, "tmux-fallback")
                    .map(PreviewState::DirectoryListing);
            }
        };

        let windows = assembly::list_session_windows(&*self.tmux, entry);
        Ok(PreviewState::TmuxScreen(
            assembly::build_tmux_screen_content(entry, windows, pane_content),
        ))
    }

    fn generate_directory_preview(&self, entry: &Entry) -> Result<PreviewState, String> {
        self.generate_directory_preview_internal(entry, "entry-path")
            .map(PreviewState::DirectoryListing)
    }

    fn generate_directory_preview_internal(
        &self,
        entry: &Entry,
        source: &str,
    ) -> Result<crate::preview::types::DirectoryListingContent, String> {
        directory::generate_directory_preview_content(&*self.tmux, entry, source, MAX_DIR_ENTRIES)
    }
}

#[cfg(test)]
fn read_directory(path: &str, max_entries: usize) -> Vec<String> {
    directory::read_directory(path, max_entries)
}

#[cfg(test)]
fn tail_lines_preserve_layout(content: &str, max_lines: usize) -> Vec<String> {
    pane_capture::tail_lines_preserve_layout(content, max_lines)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapters::tmux::{RawSession, RawWindow, TmuxSource};
    use crate::domain::entry::SortPriority;
    use crate::domain::error::{ActionError, AdapterError};

    type CaptureArgs = Option<(usize, Option<u16>, Option<u16>)>;
    type SharedCaptureArgs = std::sync::Arc<std::sync::Mutex<CaptureArgs>>;

    struct StaticPreviewTmux {
        pane: String,
        windows: Vec<RawWindow>,
        capture_fails: bool,
        last_capture_target: std::sync::Arc<std::sync::Mutex<Option<String>>>,
        last_capture_args: SharedCaptureArgs,
    }

    impl TmuxSource for StaticPreviewTmux {
        fn list_windows(&self) -> Result<Vec<RawWindow>, AdapterError> {
            Ok(self.windows.clone())
        }

        fn list_sessions(&self) -> Result<Vec<RawSession>, AdapterError> {
            Ok(vec![])
        }

        fn current_session(&self) -> Result<String, AdapterError> {
            Ok("s".to_string())
        }

        fn current_window_index(&self) -> Result<String, AdapterError> {
            Ok("0".to_string())
        }

        fn has_session(&self, _name: &str) -> Result<bool, AdapterError> {
            Ok(false)
        }

        fn select_window(&self, _target: &str) -> Result<(), ActionError> {
            Ok(())
        }

        fn new_session(&self, _name: &str, _path: &str) -> Result<(), ActionError> {
            Ok(())
        }

        fn new_window(&self, _session: &str, _path: &str) -> Result<String, ActionError> {
            Ok("s:1".to_string())
        }

        fn switch_client(&self, _target: &str) -> Result<(), ActionError> {
            Ok(())
        }

        fn kill_window(&self, _target: &str) -> Result<(), ActionError> {
            Ok(())
        }

        fn kill_session(&self, _name: &str) -> Result<(), ActionError> {
            Ok(())
        }

        fn capture_pane(&self, _target: &str, _line_count: usize) -> Result<String, AdapterError> {
            if self.capture_fails {
                return Err(AdapterError::TmuxCommand {
                    command: "capture-pane".to_string(),
                    detail: "fake capture failure".to_string(),
                });
            }
            Ok(self.pane.clone())
        }

        fn capture_pane_with_size(
            &self,
            target: &str,
            line_count: usize,
            width: Option<u16>,
            height: Option<u16>,
        ) -> Result<String, AdapterError> {
            *self
                .last_capture_target
                .lock()
                .expect("capture target mutex should not be poisoned") = Some(target.to_string());
            *self
                .last_capture_args
                .lock()
                .expect("capture args mutex should not be poisoned") =
                Some((line_count, width, height));
            if self.capture_fails {
                return Err(AdapterError::TmuxCommand {
                    command: "capture-pane".to_string(),
                    detail: "fake capture failure".to_string(),
                });
            }
            Ok(self.pane.clone())
        }
    }

    #[test]
    fn read_directory_nonexistent() {
        let result = read_directory("/nonexistent/path/xyz", 10);
        assert_eq!(result, vec!["(cannot read directory)"]);
    }

    #[test]
    fn read_directory_empty() {
        let dir = std::env::temp_dir();
        let test_dir = dir.join("tmux_popup_test_empty_dir");
        let _ = std::fs::create_dir_all(&test_dir);
        let result = read_directory(test_dir.to_str().unwrap(), 10);
        assert!(result.is_empty());
        let _ = std::fs::remove_dir(&test_dir);
    }

    #[test]
    fn read_directory_respects_limit() {
        let dir = std::env::temp_dir();
        let test_dir = dir.join("tmux_popup_test_limit");
        let _ = std::fs::create_dir_all(&test_dir);
        for i in 0..5 {
            std::fs::write(test_dir.join(format!("file{i}")), "").unwrap();
        }
        let result = read_directory(test_dir.to_str().unwrap(), 2);
        assert_eq!(result.len(), 2);
        let _ = std::fs::remove_dir_all(&test_dir);
    }

    #[test]
    fn read_directory_sorts_by_name() {
        let dir = std::env::temp_dir();
        let test_dir = dir.join("tmux_popup_test_sort");
        let _ = std::fs::create_dir_all(&test_dir);
        std::fs::write(test_dir.join("charlie"), "").unwrap();
        std::fs::write(test_dir.join("alpha"), "").unwrap();
        let result = read_directory(test_dir.to_str().unwrap(), 10);
        assert!(
            result[0].contains("alpha"),
            "first entry should contain alpha: {}",
            result[0]
        );
        assert!(
            result[1].contains("charlie"),
            "second entry should contain charlie: {}",
            result[1]
        );
        let _ = std::fs::remove_dir_all(&test_dir);
    }

    #[test]
    fn read_directory_sorts_directories_before_files() {
        let dir = std::env::temp_dir();
        let test_dir = dir.join("tmux_popup_test_dir_first");
        let _ = std::fs::create_dir_all(&test_dir);
        let _ = std::fs::create_dir(test_dir.join("z_dir"));
        std::fs::write(test_dir.join("a_file"), "").unwrap();

        let result = read_directory(test_dir.to_str().unwrap(), 10);

        assert!(result[0].contains("z_dir"));
        assert!(result[1].contains("a_file"));
        let _ = std::fs::remove_dir_all(&test_dir);
    }

    #[test]
    fn window_preview_prefers_latest_screen_lines() {
        let pane = (0..260)
            .map(|i| format!("line-{i}"))
            .collect::<Vec<_>>()
            .join("\n");
        let tmux = StaticPreviewTmux {
            pane,
            windows: vec![RawWindow {
                session_name: "s".to_string(),
                window_index: "0".to_string(),
                window_name: "main".to_string(),
                window_path: "/tmp".to_string(),
                window_activity: None,
            }],
            capture_fails: false,
            last_capture_target: std::sync::Arc::new(std::sync::Mutex::new(None)),
            last_capture_args: std::sync::Arc::new(std::sync::Mutex::new(None)),
        };

        let generator = PreviewGenerator::with_factory(Box::new(tmux), || {
            Box::new(StaticPreviewTmux {
                pane: String::new(),
                windows: vec![],
                capture_fails: false,
                last_capture_target: std::sync::Arc::new(std::sync::Mutex::new(None)),
                last_capture_args: std::sync::Arc::new(std::sync::Mutex::new(None)),
            })
        });

        let entry = Entry::window(
            "s".to_string(),
            "0".to_string(),
            "main".to_string(),
            "/tmp".to_string(),
            SortPriority::CurrentWindow,
            true,
            None,
        );

        let preview = generator
            .generate(&entry, None)
            .expect("preview should be generated");
        let pane_content = match preview {
            PreviewState::TmuxScreen(content) => content.screen_lines,
            other => panic!("expected tmux screen preview, got {other:?}"),
        };

        assert_eq!(pane_content.last().map(String::as_str), Some("line-259"));
        assert!(!pane_content.iter().any(|line| line == "line-0"));
    }

    #[test]
    fn window_preview_respects_capture_height_from_dimensions() {
        let pane = (0..30)
            .map(|i| format!("line-{i}"))
            .collect::<Vec<_>>()
            .join("\n");
        let tmux = StaticPreviewTmux {
            pane,
            windows: vec![RawWindow {
                session_name: "s".to_string(),
                window_index: "0".to_string(),
                window_name: "main".to_string(),
                window_path: "/tmp".to_string(),
                window_activity: None,
            }],
            capture_fails: false,
            last_capture_target: std::sync::Arc::new(std::sync::Mutex::new(None)),
            last_capture_args: std::sync::Arc::new(std::sync::Mutex::new(None)),
        };

        let generator = PreviewGenerator::with_factory(Box::new(tmux), || {
            Box::new(StaticPreviewTmux {
                pane: String::new(),
                windows: vec![],
                capture_fails: false,
                last_capture_target: std::sync::Arc::new(std::sync::Mutex::new(None)),
                last_capture_args: std::sync::Arc::new(std::sync::Mutex::new(None)),
            })
        });

        let entry = Entry::window(
            "s".to_string(),
            "0".to_string(),
            "main".to_string(),
            "/tmp".to_string(),
            SortPriority::CurrentWindow,
            true,
            None,
        );

        let preview = generator
            .generate(&entry, Some((120, 5)))
            .expect("preview should be generated");
        let pane_content = match preview {
            PreviewState::TmuxScreen(content) => content.screen_lines,
            other => panic!("expected tmux screen preview, got {other:?}"),
        };

        assert_eq!(pane_content.len(), 5);
        assert_eq!(pane_content.first().map(String::as_str), Some("line-25"));
        assert_eq!(pane_content.last().map(String::as_str), Some("line-29"));
    }

    #[test]
    fn tail_lines_preserve_layout_keeps_blank_and_indented_lines() {
        let content = "line-1\n    indented\n\nline-4\n";

        let tailed = tail_lines_preserve_layout(content, 4);

        assert_eq!(tailed, vec!["line-1", "    indented", "", "line-4"]);
    }

    #[test]
    fn tail_lines_preserve_layout_does_not_drop_real_last_line_when_content_ends_with_newline() {
        let content = "a\nb\nc\n";

        let tailed = tail_lines_preserve_layout(content, 2);

        assert_eq!(tailed, vec!["b", "c"]);
    }

    #[test]
    fn window_preview_passes_dimensions_to_tmux_capture() {
        let capture_args = std::sync::Arc::new(std::sync::Mutex::new(None));
        let tmux = StaticPreviewTmux {
            pane: "line-a\nline-b\nline-c".to_string(),
            windows: vec![RawWindow {
                session_name: "s".to_string(),
                window_index: "0".to_string(),
                window_name: "main".to_string(),
                window_path: "/tmp".to_string(),
                window_activity: None,
            }],
            capture_fails: false,
            last_capture_target: std::sync::Arc::new(std::sync::Mutex::new(None)),
            last_capture_args: capture_args.clone(),
        };

        let generator = PreviewGenerator::with_factory(Box::new(tmux), || {
            Box::new(StaticPreviewTmux {
                pane: String::new(),
                windows: vec![],
                capture_fails: false,
                last_capture_target: std::sync::Arc::new(std::sync::Mutex::new(None)),
                last_capture_args: std::sync::Arc::new(std::sync::Mutex::new(None)),
            })
        });

        let entry = Entry::window(
            "s".to_string(),
            "0".to_string(),
            "main".to_string(),
            "/tmp".to_string(),
            SortPriority::CurrentWindow,
            true,
            None,
        );

        let _ = generator
            .generate(&entry, Some((88, 9)))
            .expect("preview should be generated");

        let args = capture_args
            .lock()
            .expect("capture args mutex should not be poisoned")
            .as_ref()
            .copied()
            .expect("capture args should be recorded");

        assert_eq!(args, (9, Some(88), Some(9)));
    }

    #[test]
    fn window_preview_uses_entry_target_for_tmux_capture() {
        let capture_target = std::sync::Arc::new(std::sync::Mutex::new(None));
        let tmux = StaticPreviewTmux {
            pane: "line-a\nline-b\nline-c".to_string(),
            windows: vec![RawWindow {
                session_name: "my-session".to_string(),
                window_index: "7".to_string(),
                window_name: "main".to_string(),
                window_path: "/tmp".to_string(),
                window_activity: None,
            }],
            capture_fails: false,
            last_capture_target: capture_target.clone(),
            last_capture_args: std::sync::Arc::new(std::sync::Mutex::new(None)),
        };

        let generator = PreviewGenerator::with_factory(Box::new(tmux), || {
            Box::new(StaticPreviewTmux {
                pane: String::new(),
                windows: vec![],
                capture_fails: false,
                last_capture_target: std::sync::Arc::new(std::sync::Mutex::new(None)),
                last_capture_args: std::sync::Arc::new(std::sync::Mutex::new(None)),
            })
        });

        let entry = Entry::window(
            "my-session".to_string(),
            "7".to_string(),
            "main".to_string(),
            "/tmp".to_string(),
            SortPriority::CurrentSessionOtherWindow,
            false,
            None,
        );

        let _ = generator
            .generate(&entry, Some((100, 6)))
            .expect("preview should be generated");

        let target = capture_target
            .lock()
            .expect("capture target mutex should not be poisoned")
            .clone()
            .expect("capture target should be recorded");

        assert_eq!(target, "my-session:7");
    }

    #[test]
    fn window_preview_without_dimensions_uses_default_capture_contract() {
        let capture_args = std::sync::Arc::new(std::sync::Mutex::new(None));
        let tmux = StaticPreviewTmux {
            pane: "l1\nl2\nl3".to_string(),
            windows: vec![RawWindow {
                session_name: "s".to_string(),
                window_index: "0".to_string(),
                window_name: "main".to_string(),
                window_path: "/tmp".to_string(),
                window_activity: None,
            }],
            capture_fails: false,
            last_capture_target: std::sync::Arc::new(std::sync::Mutex::new(None)),
            last_capture_args: capture_args.clone(),
        };

        let generator = PreviewGenerator::with_factory(Box::new(tmux), || {
            Box::new(StaticPreviewTmux {
                pane: String::new(),
                windows: vec![],
                capture_fails: false,
                last_capture_target: std::sync::Arc::new(std::sync::Mutex::new(None)),
                last_capture_args: std::sync::Arc::new(std::sync::Mutex::new(None)),
            })
        });

        let entry = Entry::window(
            "s".to_string(),
            "0".to_string(),
            "main".to_string(),
            "/tmp".to_string(),
            SortPriority::CurrentWindow,
            true,
            None,
        );

        let _ = generator
            .generate(&entry, None)
            .expect("preview should be generated");

        let args = capture_args
            .lock()
            .expect("capture args mutex should not be poisoned")
            .as_ref()
            .copied()
            .expect("capture args should be recorded");

        assert_eq!(args, (MAX_PANE_LINES, None, None));
    }

    #[test]
    fn window_preview_zero_height_dimensions_are_clamped_for_line_count() {
        let capture_args = std::sync::Arc::new(std::sync::Mutex::new(None));
        let tmux = StaticPreviewTmux {
            pane: "only-one-line".to_string(),
            windows: vec![RawWindow {
                session_name: "s".to_string(),
                window_index: "0".to_string(),
                window_name: "main".to_string(),
                window_path: "/tmp".to_string(),
                window_activity: None,
            }],
            capture_fails: false,
            last_capture_target: std::sync::Arc::new(std::sync::Mutex::new(None)),
            last_capture_args: capture_args.clone(),
        };

        let generator = PreviewGenerator::with_factory(Box::new(tmux), || {
            Box::new(StaticPreviewTmux {
                pane: String::new(),
                windows: vec![],
                capture_fails: false,
                last_capture_target: std::sync::Arc::new(std::sync::Mutex::new(None)),
                last_capture_args: std::sync::Arc::new(std::sync::Mutex::new(None)),
            })
        });

        let entry = Entry::window(
            "s".to_string(),
            "0".to_string(),
            "main".to_string(),
            "/tmp".to_string(),
            SortPriority::CurrentWindow,
            true,
            None,
        );

        let _ = generator
            .generate(&entry, Some((120, 0)))
            .expect("preview should be generated");

        let args = capture_args
            .lock()
            .expect("capture args mutex should not be poisoned")
            .as_ref()
            .copied()
            .expect("capture args should be recorded");

        assert_eq!(args, (1, Some(120), Some(0)));
    }

    #[test]
    fn directory_preview_uses_native_read_dir() {
        let tmux = StaticPreviewTmux {
            pane: String::new(),
            windows: vec![],
            capture_fails: false,
            last_capture_target: std::sync::Arc::new(std::sync::Mutex::new(None)),
            last_capture_args: std::sync::Arc::new(std::sync::Mutex::new(None)),
        };
        let generator = PreviewGenerator::with_factory_for_test(Box::new(tmux), || {
            Box::new(StaticPreviewTmux {
                pane: String::new(),
                windows: vec![],
                capture_fails: false,
                last_capture_target: std::sync::Arc::new(std::sync::Mutex::new(None)),
                last_capture_args: std::sync::Arc::new(std::sync::Mutex::new(None)),
            })
        });

        let test_dir = std::env::temp_dir().join("tmux_native_dir_test");
        let _ = std::fs::create_dir_all(&test_dir);
        std::fs::write(test_dir.join("visible.txt"), "").unwrap();
        std::fs::write(test_dir.join(".hidden"), "").unwrap();

        let entry = Entry::zoxide(
            "test-project".to_string(),
            test_dir.to_str().unwrap().to_string(),
        );
        let preview = generator
            .generate(&entry, None)
            .expect("preview should be generated");

        match preview {
            PreviewState::DirectoryListing(content) => {
                assert!(content.entries.iter().any(|d| d.contains("visible.txt")));
                assert!(!content.entries.iter().any(|d| d.contains(".hidden")));
            }
            other => panic!("expected DirectoryListing preview, got {other:?}"),
        }

        let _ = std::fs::remove_dir_all(&test_dir);
    }

    #[test]
    fn read_directory_hides_dotfiles() {
        let dir = std::env::temp_dir();
        let test_dir = dir.join("tmux_popup_test_hidden");
        let _ = std::fs::create_dir_all(&test_dir);
        std::fs::write(test_dir.join("visible"), "").unwrap();
        std::fs::write(test_dir.join(".gitignore"), "").unwrap();
        std::fs::write(test_dir.join(".hidden"), "").unwrap();

        let result = read_directory(test_dir.to_str().unwrap(), 10);
        assert_eq!(result.len(), 1);
        assert!(result[0].contains("visible"));
        let _ = std::fs::remove_dir_all(&test_dir);
    }

    #[test]
    fn window_preview_falls_back_to_directory_listing_when_capture_fails() {
        let test_dir = std::env::temp_dir().join("tmux_window_preview_capture_fallback");
        let _ = std::fs::create_dir_all(&test_dir);
        std::fs::write(test_dir.join("visible.txt"), "").unwrap();

        let tmux = StaticPreviewTmux {
            pane: String::new(),
            windows: vec![],
            capture_fails: true,
            last_capture_target: std::sync::Arc::new(std::sync::Mutex::new(None)),
            last_capture_args: std::sync::Arc::new(std::sync::Mutex::new(None)),
        };
        let generator = PreviewGenerator::with_factory_for_test(Box::new(tmux), || {
            Box::new(StaticPreviewTmux {
                pane: String::new(),
                windows: vec![],
                capture_fails: false,
                last_capture_target: std::sync::Arc::new(std::sync::Mutex::new(None)),
                last_capture_args: std::sync::Arc::new(std::sync::Mutex::new(None)),
            })
        });

        let entry = Entry::window(
            "missing-session".to_string(),
            "99".to_string(),
            "ghost".to_string(),
            test_dir.to_str().unwrap().to_string(),
            SortPriority::OtherSessionWindow,
            false,
            None,
        );

        let preview = generator
            .generate(&entry, Some((120, 10)))
            .expect("fallback directory listing should be generated");
        match preview {
            PreviewState::DirectoryListing(content) => {
                assert_eq!(content.path, test_dir.to_str().unwrap().to_string());
                assert_eq!(content.source, "tmux-fallback");
                assert!(content
                    .entries
                    .iter()
                    .any(|line| line.contains("visible.txt")));
            }
            other => panic!("expected directory listing fallback, got {other:?}"),
        }

        let _ = std::fs::remove_dir_all(&test_dir);
    }

    #[test]
    fn window_preview_falls_back_to_directory_listing_when_target_invalid() {
        let test_dir = std::env::temp_dir().join("tmux_invalid_target_fallback_test");
        let _ = std::fs::create_dir_all(&test_dir);
        std::fs::write(test_dir.join("readme.md"), "").unwrap();
        std::fs::write(test_dir.join("src"), "").unwrap();

        let tmux = StaticPreviewTmux {
            pane: String::new(),
            windows: vec![],
            capture_fails: true,
            last_capture_target: std::sync::Arc::new(std::sync::Mutex::new(None)),
            last_capture_args: std::sync::Arc::new(std::sync::Mutex::new(None)),
        };
        let generator = PreviewGenerator::with_factory_for_test(Box::new(tmux), || {
            Box::new(StaticPreviewTmux {
                pane: String::new(),
                windows: vec![],
                capture_fails: false,
                last_capture_target: std::sync::Arc::new(std::sync::Mutex::new(None)),
                last_capture_args: std::sync::Arc::new(std::sync::Mutex::new(None)),
            })
        });

        let entry = Entry::window(
            "nonexistent".to_string(),
            "999".to_string(),
            "invalid-target".to_string(),
            test_dir.to_str().unwrap().to_string(),
            SortPriority::OtherSessionWindow,
            false,
            None,
        );

        let preview = generator
            .generate(&entry, Some((80, 12)))
            .expect("should fallback to directory listing when capture returns Err");

        match preview {
            PreviewState::DirectoryListing(content) => {
                assert_eq!(content.source, "tmux-fallback");
                assert_eq!(content.path, test_dir.to_str().unwrap().to_string());
                assert_eq!(
                    content.name,
                    test_dir.file_name().unwrap().to_str().unwrap()
                );
                assert!(
                    !content.entries.is_empty(),
                    "directory listing should have entries"
                );
                assert!(content.entries.iter().any(|e| e.contains("readme.md")));
                assert!(content.entries.iter().any(|e| e.contains("src")));
            }
            other => panic!("expected DirectoryListing fallback, got {other:?}"),
        }

        let _ = std::fs::remove_dir_all(&test_dir);
    }

    #[test]
    fn directory_preview_with_native_listing() {
        let tmux = StaticPreviewTmux {
            pane: String::new(),
            windows: vec![],
            capture_fails: false,
            last_capture_target: std::sync::Arc::new(std::sync::Mutex::new(None)),
            last_capture_args: std::sync::Arc::new(std::sync::Mutex::new(None)),
        };
        let generator = PreviewGenerator::with_factory_for_test(Box::new(tmux), || {
            Box::new(StaticPreviewTmux {
                pane: String::new(),
                windows: vec![],
                capture_fails: false,
                last_capture_target: std::sync::Arc::new(std::sync::Mutex::new(None)),
                last_capture_args: std::sync::Arc::new(std::sync::Mutex::new(None)),
            })
        });

        let test_dir = std::env::temp_dir().join("tmux_native_listing_test");
        let _ = std::fs::create_dir_all(test_dir.join("subdir"));
        std::fs::write(test_dir.join("file.txt"), "").unwrap();
        std::fs::write(test_dir.join("subdir").join("nested.txt"), "").unwrap();
        std::fs::write(test_dir.join(".hidden"), "").unwrap();

        let entry = Entry::zoxide(
            "native-listing-test-project".to_string(),
            test_dir.to_str().unwrap().to_string(),
        );
        let preview = generator
            .generate(&entry, None)
            .expect("preview should be generated");

        match preview {
            PreviewState::DirectoryListing(content) => {
                assert!(content.entries.iter().any(|d| d.contains("subdir")));
                assert!(content.entries.iter().any(|d| d.contains("file.txt")));
                assert_eq!(content.source, "entry-path");
                assert!(!content.entries.iter().any(|d| d.contains(".hidden")));
            }
            other => panic!("expected DirectoryListing preview, got {other:?}"),
        }

        let _ = std::fs::remove_dir_all(&test_dir);
    }

    #[test]
    fn window_preview_returns_error_when_capture_fails_and_path_invalid() {
        let tmux = StaticPreviewTmux {
            pane: String::new(),
            windows: vec![],
            capture_fails: true,
            last_capture_target: std::sync::Arc::new(std::sync::Mutex::new(None)),
            last_capture_args: std::sync::Arc::new(std::sync::Mutex::new(None)),
        };
        let generator = PreviewGenerator::with_factory(Box::new(tmux), || {
            Box::new(StaticPreviewTmux {
                pane: String::new(),
                windows: vec![],
                capture_fails: false,
                last_capture_target: std::sync::Arc::new(std::sync::Mutex::new(None)),
                last_capture_args: std::sync::Arc::new(std::sync::Mutex::new(None)),
            })
        });

        let entry = Entry::window(
            "missing-session".to_string(),
            "99".to_string(),
            "ghost".to_string(),
            "/nonexistent/path/for/tmux-preview-fallback".to_string(),
            SortPriority::OtherSessionWindow,
            false,
            None,
        );

        let result = generator.generate(&entry, Some((100, 8)));
        assert!(result.is_err());
    }
}
