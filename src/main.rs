pub mod app_state;
mod dap_logger;
mod dap_server;
mod log_search;

#[macro_use]
extern crate log;

use crate::app_state::App;
use anyhow::{anyhow, Context, Error, Result};
fn main() {
    dap_logger::init();
    log_panics::init();

    let app = App::init();
    app.app_loop();
}
