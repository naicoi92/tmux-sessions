use std::sync::atomic::{AtomicU64, Ordering};

static ID_COUNTER: AtomicU64 = AtomicU64::new(0);

pub fn next_id() -> u64 {
    ID_COUNTER.fetch_add(1, Ordering::Relaxed)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PreviewState {
    TmuxScreen(TmuxScreenContent),
    DirectoryListing(DirectoryListingContent),
    #[deprecated(note = "Use PreviewState::TmuxScreen instead")]
    Summary(PreviewContent),
    #[deprecated(note = "Use PreviewState::DirectoryListing instead")]
    CreateIntent(PreviewContent),
    Loading,
    Error(String),
    Empty,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TmuxScreenContent {
    pub session_name: String,
    pub path: String,
    pub target: String,
    pub windows: Vec<String>,
    pub screen_lines: Vec<String>,
    pub is_fallback: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DirectoryListingContent {
    pub name: String,
    pub path: String,
    pub headline: String,
    pub entries: Vec<String>,
    pub has_session: bool,
    pub source: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PreviewContent {
    #[deprecated(note = "Legacy migration type. Use explicit PreviewState content structs.")]
    Contextual {
        name: String,
        path: String,
        headline: String,
        details: Vec<String>,
        pane_content: Option<Vec<String>>,
    },
}
