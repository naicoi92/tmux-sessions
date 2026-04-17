use crate::adapters::tmux::raw::{RawSession, RawWindow};
use crate::adapters::tmux::TmuxSource;
use crate::domain::error::{ActionError, AdapterError};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FakeTmuxCall {
    SelectWindow(String),
    NewSession { name: String, path: String },
    SwitchClient(String),
    KillWindow(String),
    KillSession(String),
}

pub struct FakeTmuxSource {
    pub windows: Vec<RawWindow>,
    pub sessions: Vec<RawSession>,
    pub current_session_name: String,
    pub current_window_idx: String,
    pub existing_sessions: Vec<String>,
    pub fail_on: Vec<FakeTmuxCall>,
}

impl FakeTmuxSource {
    pub fn new() -> Self {
        Self {
            windows: vec![],
            sessions: vec![],
            current_session_name: "default".into(),
            current_window_idx: "0".into(),
            existing_sessions: vec![],
            fail_on: vec![],
        }
    }

    pub fn with_window(session: &str, index: &str, name: &str, path: &str) -> Self {
        Self {
            windows: vec![RawWindow {
                session_name: session.into(),
                window_index: index.into(),
                window_name: name.into(),
                window_path: path.into(),
                window_activity: None,
            }],
            sessions: vec![RawSession {
                session_name: session.into(),
                attached: true,
            }],
            current_session_name: session.into(),
            current_window_idx: index.into(),
            existing_sessions: vec![session.into()],
            fail_on: vec![],
        }
    }
}

impl Default for FakeTmuxSource {
    fn default() -> Self {
        Self::new()
    }
}

impl TmuxSource for FakeTmuxSource {
    fn list_windows(&self) -> Result<Vec<RawWindow>, AdapterError> {
        Ok(self.windows.clone())
    }
    fn list_sessions(&self) -> Result<Vec<RawSession>, AdapterError> {
        Ok(self.sessions.clone())
    }
    fn current_session(&self) -> Result<String, AdapterError> {
        Ok(self.current_session_name.clone())
    }
    fn current_window_index(&self) -> Result<String, AdapterError> {
        Ok(self.current_window_idx.clone())
    }
    fn has_session(&self, name: &str) -> Result<bool, AdapterError> {
        Ok(self.existing_sessions.iter().any(|s| s == name))
    }
    fn select_window(&self, target: &str) -> Result<(), ActionError> {
        if self
            .fail_on
            .contains(&FakeTmuxCall::SelectWindow(target.to_string()))
        {
            return Err(ActionError::GotoFailed {
                target: target.to_string(),
                detail: "fake failure".to_string(),
            });
        }
        Ok(())
    }
    fn new_session(&self, name: &str, path: &str) -> Result<(), ActionError> {
        if self.fail_on.contains(&FakeTmuxCall::NewSession {
            name: name.to_string(),
            path: path.to_string(),
        }) {
            return Err(ActionError::GotoFailed {
                target: name.to_string(),
                detail: "fake failure".to_string(),
            });
        }
        Ok(())
    }
    fn new_window(&self, session: &str, _path: &str) -> Result<String, ActionError> {
        Ok(format!("{session}:99"))
    }
    fn switch_client(&self, target: &str) -> Result<(), ActionError> {
        if self
            .fail_on
            .contains(&FakeTmuxCall::SwitchClient(target.to_string()))
        {
            return Err(ActionError::GotoFailed {
                target: target.to_string(),
                detail: "fake failure".to_string(),
            });
        }
        Ok(())
    }
    fn kill_window(&self, target: &str) -> Result<(), ActionError> {
        if self
            .fail_on
            .contains(&FakeTmuxCall::KillWindow(target.to_string()))
        {
            return Err(ActionError::KillFailed {
                target: target.to_string(),
                detail: "fake failure".to_string(),
            });
        }
        Ok(())
    }
    fn kill_session(&self, name: &str) -> Result<(), ActionError> {
        if self
            .fail_on
            .contains(&FakeTmuxCall::KillSession(name.to_string()))
        {
            return Err(ActionError::KillFailed {
                target: name.to_string(),
                detail: "fake failure".to_string(),
            });
        }
        Ok(())
    }
    fn capture_pane(&self, target: &str, _line_count: usize) -> Result<String, AdapterError> {
        Err(AdapterError::TmuxCommand {
            command: format!("capture-pane -t {target}"),
            detail: "fake capture unavailable".to_string(),
        })
    }

    fn capture_pane_with_size(
        &self,
        target: &str,
        _line_count: usize,
        _width: Option<u16>,
        _height: Option<u16>,
    ) -> Result<String, AdapterError> {
        Err(AdapterError::TmuxCommand {
            command: format!("capture-pane -t {target}"),
            detail: "fake capture unavailable".to_string(),
        })
    }
}
