#[derive(Clone, Debug)]
pub struct RawWindow {
    pub session_name: String,
    pub window_index: String,
    pub window_name: String,
    pub window_path: String,
    /// Fixed path set via `@pi_original_path` at creation time.
    /// When present, used instead of `window_path` (which is `pane_current_path`
    /// and changes on `cd`).
    pub original_path: Option<String>,
    pub window_activity: Option<i64>,
}

#[derive(Clone)]
pub struct RawSession {
    pub session_name: String,
    pub attached: bool,
    pub session_activity: Option<i64>,
}
