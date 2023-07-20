pub mod app_state;
mod dap_logger;
mod dap_server;
mod log_search;

#[macro_use]
extern crate log;

use crate::app_state::{App, AppState};
use anyhow::{anyhow, Context, Error, Result};
fn main() {
    dap_logger::init();
    log_panics::init();

    let mut app = App::init();
    while !matches!(app.state, AppState::Exit) {
        app.app_loop();
    }
}
