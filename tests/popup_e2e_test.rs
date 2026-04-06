use tmux_sessions::app::controller::{init_terminal, restore_terminal, ExitAction};
use tmux_sessions::app::executor::ExitReason;

#[test]
fn exit_action_quit_variant() {
    assert!(matches!(ExitAction::Quit, ExitAction::Quit));
}

#[test]
fn exit_action_switch_to_variant() {
    match ExitAction::SwitchTo("s:0".into()) {
        ExitAction::SwitchTo(target) => assert_eq!(target, "s:0"),
        ExitAction::Quit => panic!("expected SwitchTo"),
    }
}

#[test]
fn exit_action_equality() {
    assert_eq!(ExitAction::Quit, ExitAction::Quit);
    assert_eq!(
        ExitAction::SwitchTo("s:0".into()),
        ExitAction::SwitchTo("s:0".into())
    );
    assert_ne!(
        ExitAction::SwitchTo("s:0".into()),
        ExitAction::SwitchTo("s:1".into())
    );
    assert_ne!(ExitAction::Quit, ExitAction::SwitchTo("s:0".into()));
}

#[test]
fn exit_reason_send_sync() {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<ExitReason>();
}

#[test]
fn exit_action_send_sync() {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<ExitAction>();
}

#[test]
fn restore_terminal_idempotent() {
    restore_terminal().ok();
    restore_terminal().ok();
}

#[test]
fn init_terminal_restores_cleanly() {
    match init_terminal() {
        Ok(_terminal) => {
            assert!(
                restore_terminal().is_ok(),
                "restore after init should succeed"
            );
        }
        Err(_) => {
            restore_terminal().ok();
        }
    }
}
