use dap::{
    base_message::Sendable, events::Event, events::OutputEventBody, types::OutputEventCategory,
};
use log::{self, Level, LevelFilter, Metadata, Record};

use crate::dap_server;

struct DAPLogger {}
static LOGGER: DAPLogger = DAPLogger {};

pub fn init() -> Result<(), log::SetLoggerError> {
    log::set_logger(&LOGGER).map(|()| log::set_max_level(LevelFilter::Info))
}

impl log::Log for DAPLogger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() <= Level::Info
    }

    fn log(&self, record: &Record) {
        if self.enabled(record.metadata()) {
            dap_server::write(Sendable::Event(Event::Output(OutputEventBody {
                category: Some(OutputEventCategory::Console),
                output: format!("[{}] {}\n", record.level(), record.args()),
                ..Default::default()
            })));
        }
    }

    fn flush(&self) {}
}
