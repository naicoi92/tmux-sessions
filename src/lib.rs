pub mod adapters;
pub mod app;
pub mod domain;
pub mod preview;
pub mod ui;

use app::controller::ExitAction;
use app::loader::{create_debug_loader, create_production_loader};

pub fn run_tui(debug: bool) -> Result<ExitAction, Box<dyn std::error::Error>> {
    let loader = if debug {
        create_debug_loader()
    } else {
        create_production_loader()
    };
    app::controller::run_app(loader, debug)
}
