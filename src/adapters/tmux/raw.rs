#[derive(Clone, Debug)]
pub struct RawWindow {
    pub session_name: String,
    pub window_index: String,
    pub window_name: String,
    pub window_path: String,
    pub window_activity: Option<i64>,
}

#[derive(Clone)]
pub struct RawSession {
    pub session_name: String,
    pub attached: bool,
}
