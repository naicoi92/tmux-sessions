use std::env;
use std::process::{self, Command};

const VERSION: &str = env!("CARGO_PKG_VERSION");

enum TmuxContext {
    OutsideTmux,
    TmuxPane,
    TmuxPopup,
}

fn detect_tmux_context() -> TmuxContext {
    let has_tmux = env::var("TMUX").is_ok();
    let has_pane = env::var("TMUX_PANE").is_ok();

    match (has_tmux, has_pane) {
        (false, _) => TmuxContext::OutsideTmux,
        (true, false) => TmuxContext::TmuxPopup,
        (true, true) => TmuxContext::TmuxPane,
    }
}

fn attach_main_session() -> ! {
    let status = Command::new("tmux")
        .args(["new-session", "-A", "-s", "main"])
        .status();
    match status {
        Ok(s) if s.success() => process::exit(0),
        Ok(s) => {
            eprintln!("error: failed to start tmux main session (exit: {s})");
            process::exit(1);
        }
        Err(e) => {
            eprintln!("error: cannot execute tmux new-session -A -s main: {e}");
            process::exit(1);
        }
    }
}

fn launch_popup() -> ! {
    let exe = match env::current_exe() {
        Ok(p) => p,
        Err(e) => {
            eprintln!("error: cannot determine executable path: {e}");
            process::exit(1);
        }
    };

    let status = Command::new("tmux")
        .args([
            "display-popup",
            "-h",
            "80%",
            "-w",
            "80%",
            "-E",
            &exe.display().to_string(),
        ])
        .status();

    match status {
        Ok(s) if s.success() => process::exit(0),
        Ok(s) => {
            eprintln!("error: display-popup failed (exit: {s})");
            process::exit(1);
        }
        Err(e) => {
            eprintln!("error: cannot execute tmux display-popup: {e}");
            process::exit(1);
        }
    }
}

fn run_tui(debug: bool) -> ! {
    let result = tmux_sessions::run_tui(debug);

    match result {
        Ok(tmux_sessions::app::controller::ExitAction::SwitchTo(target)) => {
            if debug {
                eprintln!("[debug] switch to: {target}");
            }
            process::exit(0);
        }
        Ok(tmux_sessions::app::controller::ExitAction::Quit) => process::exit(0),
        Err(e) => {
            eprintln!("error: {e}");
            process::exit(1);
        }
    }
}

fn print_usage() {
    println!("tmux-sessions {VERSION}");
    println!();
    println!("USAGE:");
    println!("    tmux-sessions [OPTIONS]");
    println!();
    println!("OPTIONS:");
    println!("    -h, --help       Print help information");
    println!("    -V, --version    Print version information");
    println!("    --debug          Run in debug mode (no tmux calls)");
    println!();
    println!("BEHAVIOR:");
    println!("    Outside tmux  → attach to 'main' session (create if needed)");
    println!("    In tmux pane → open popup (display-popup)");
    println!("    In popup     → run TUI");
    println!();
    println!("TMUX BINDING:");
    println!("    bind-key -n M-w display-popup -h 80% -w 80% -E \"ts\"");
}

fn install_panic_hook() {
    let original_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let _ = crossterm::terminal::disable_raw_mode();
        let _ = crossterm::execute!(std::io::stdout(), crossterm::terminal::LeaveAlternateScreen);
        original_hook(info);
    }));
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let debug = args.contains(&"--debug".to_string());

    let filtered: Vec<&str> = args
        .iter()
        .map(String::as_str)
        .filter(|a| *a != "--debug")
        .collect();

    if filtered.len() > 1 {
        match filtered[1] {
            "-h" | "--help" => {
                print_usage();
                process::exit(0);
            }
            "-V" | "--version" => {
                println!("tmux-sessions {VERSION}");
                process::exit(0);
            }
            unknown => {
                eprintln!("error: unknown option: {unknown}");
                print_usage();
                process::exit(1);
            }
        }
    }

    install_panic_hook();

    if debug {
        run_tui(true);
    }

    match detect_tmux_context() {
        TmuxContext::OutsideTmux => attach_main_session(),
        TmuxContext::TmuxPane => launch_popup(),
        TmuxContext::TmuxPopup => run_tui(false),
    }
}
