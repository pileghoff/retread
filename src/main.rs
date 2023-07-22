pub mod app_state;
mod dap_logger;
mod dap_server;
mod log_search;

#[macro_use]
extern crate log;

use crate::app_state::{App, AppState};
fn main() {
    dap_logger::init().unwrap();
    log_panics::init();

    let mut app = App::init();
    while !matches!(app.state, AppState::Exit) {
        if let Err(e) = app.app_loop() {
            error!("{}", e);
        }
    }
}
