use std::fmt;

#[derive(Debug)]
pub enum AdapterError {
    TmuxCommand { command: String, detail: String },
    TmuxParse { input: String, detail: String },
    ZoxideCommand { command: String, detail: String },
    ZoxideParse { input: String, detail: String },
}

#[derive(Debug)]
pub enum ActionError {
    GotoFailed { target: String, detail: String },
    KillFailed { target: String, detail: String },
    SessionCheckFailed { name: String, detail: String },
}

impl fmt::Display for AdapterError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TmuxCommand { command, detail } => {
                write!(f, "tmux command failed [{command}]: {detail}")
            }
            Self::TmuxParse { input, detail } => {
                write!(f, "tmux parse failed [{input}]: {detail}")
            }
            Self::ZoxideCommand { command, detail } => {
                write!(f, "zoxide command failed [{command}]: {detail}")
            }
            Self::ZoxideParse { input, detail } => {
                write!(f, "zoxide parse failed [{input}]: {detail}")
            }
        }
    }
}

impl fmt::Display for ActionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::GotoFailed { target, detail } => {
                write!(f, "goto failed for [{target}]: {detail}")
            }
            Self::KillFailed { target, detail } => {
                write!(f, "kill failed for [{target}]: {detail}")
            }
            Self::SessionCheckFailed { name, detail } => {
                write!(f, "session check failed for [{name}]: {detail}")
            }
        }
    }
}

impl std::error::Error for AdapterError {}
impl std::error::Error for ActionError {}

impl From<std::io::Error> for AdapterError {
    fn from(err: std::io::Error) -> Self {
        Self::TmuxCommand {
            command: "io".to_string(),
            detail: err.to_string(),
        }
    }
}

impl From<std::string::FromUtf8Error> for AdapterError {
    fn from(err: std::string::FromUtf8Error) -> Self {
        Self::TmuxParse {
            input: "utf8".to_string(),
            detail: err.to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn adapter_error_display_tmux_command() {
        let err = AdapterError::TmuxCommand {
            command: "list-windows".to_string(),
            detail: "exit code 1".to_string(),
        };
        assert!(err.to_string().contains("list-windows"));
        assert!(err.to_string().contains("exit code 1"));
    }

    #[test]
    fn adapter_error_display_zoxide_parse() {
        let err = AdapterError::ZoxideParse {
            input: "bad line".to_string(),
            detail: "missing tab".to_string(),
        };
        assert!(err.to_string().contains("bad line"));
    }

    #[test]
    fn action_error_display_goto() {
        let err = ActionError::GotoFailed {
            target: "session:1".to_string(),
            detail: "no such window".to_string(),
        };
        assert!(err.to_string().contains("session:1"));
    }

    #[test]
    fn action_error_display_kill() {
        let err = ActionError::KillFailed {
            target: "mysession".to_string(),
            detail: "session not found".to_string(),
        };
        assert!(err.to_string().contains("mysession"));
    }

    #[test]
    fn action_error_display_session_check() {
        let err = ActionError::SessionCheckFailed {
            name: "test".to_string(),
            detail: "tmux error".to_string(),
        };
        assert!(err.to_string().contains("test"));
    }

    #[test]
    fn error_is_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<AdapterError>();
        assert_send_sync::<ActionError>();
    }
}
