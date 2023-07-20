pub mod app_state;
mod dap_logger;
mod dap_server;
mod log_search;

use std::time::Instant;

use dap::requests::Command::*;
use dap::requests::*;
use serde_json::Map;

use dap::base_message::Sendable;
use dap::requests::*;
use dap::responses::*;
use dap::types::*;
use dap::{events::*, requests};
use regex::Regex;
use serde_json::Value;

use crate::app_state::*;
use crate::log_search::*;

#[macro_use]
extern crate log;

fn main() {
    test_breakpoints_10_lines();
    test_breakpoints_10_lines_and_func();
}

fn test_breakpoints_10_lines_and_func() {
    println!("\n\nRunning test: 10 lines until breakpoint. Filename+Line enabled.");
    let mut app = App::init();

    let req = Request {
        seq: 1,
        command: Initialize(InitializeArguments {
            client_id: Some("test".to_string()),
            client_name: Some("test".to_string()),
            adapter_id: "retread".to_string(),
            locale: Some("en".to_string()),
            lines_start_at1: Some(true),
            columns_start_at1: Some(true),
            path_format: Some(PathFormat::Path),
            supports_variable_type: Some(true),
            supports_variable_paging: Some(true),
            supports_run_in_terminal_request: Some(true),
            supports_memory_references: Some(true),
            supports_progress_reporting: Some(true),
            supports_invalidated_event: Some(true),
            supports_memory_event: Some(true),
            supports_args_can_be_interpreted_by_shell: Some(true),
        }),
    };
    dap_server::write_server(req);
    let mut additional_data = serde_json::Map::new();
    additional_data.insert(
        "include".to_string(),
        serde_json::Value::String("./linux/**/*.c".to_string()),
    );
    additional_data.insert(
        "log_file".to_string(),
        serde_json::Value::String("./linux.log".to_string()),
    );
    additional_data.insert(
        "log_pattern".to_string(),
        serde_json::Value::String(
            "\\[(?P<file>[^:]+):(?P<line>\\d+)\\] (?P<message>.*)$".to_string(),
        ),
    );
    let req = Request {
        seq: 2,
        command: Launch(LaunchRequestArguments {
            no_debug: None,
            restart_data: None,
            additional_data: Some(serde_json::Value::Object(additional_data)),
        }),
    };

    dap_server::write_server(req);
    let req = Request {
        seq: 3,
        command: SetBreakpoints(SetBreakpointsArguments {
            source: dap::types::Source {
                name: Some("linux.log".to_string()),
                path: Some("./linux.log".to_string()),
                source_reference: None,
                presentation_hint: None,
                origin: None,
                sources: None,
                adapter_data: None,
                checksums: None,
            },
            breakpoints: Some(vec![SourceBreakpoint {
                line: 10,
                column: None,
                condition: None,
                hit_condition: None,
                log_message: None,
            }]),
            lines: None,
            source_modified: Some(false),
        }),
    };
    dap_server::write_server(req);
    let req = Request {
        seq: 4,
        command: Continue(ContinueArguments {
            thread_id: 0,
            single_thread: None,
        }),
    };
    loop {
        app.app_loop();
        if dap_server::read_server().is_none() {
            break;
        }
    }

    dap_server::write_server(req);

    let start = Instant::now();

    loop {
        app.app_loop();
        if let Some(Sendable::Event(Event::Stopped(stop_event))) = dap_server::read_server() {
            println!("Got: {:?}", stop_event.description);
            break;
        }
    }

    let end = start.elapsed();
    println!("Time taken: {:.2}s", end.as_secs_f32());
}

fn test_breakpoints_10_lines() {
    println!("\n\nRunning test: 10 lines until breakpoint. Line numbers enabled only");
    let mut app = App::init();

    let req = Request {
        seq: 1,
        command: Initialize(InitializeArguments {
            client_id: Some("test".to_string()),
            client_name: Some("test".to_string()),
            adapter_id: "retread".to_string(),
            locale: Some("en".to_string()),
            lines_start_at1: Some(true),
            columns_start_at1: Some(true),
            path_format: Some(PathFormat::Path),
            supports_variable_type: Some(true),
            supports_variable_paging: Some(true),
            supports_run_in_terminal_request: Some(true),
            supports_memory_references: Some(true),
            supports_progress_reporting: Some(true),
            supports_invalidated_event: Some(true),
            supports_memory_event: Some(true),
            supports_args_can_be_interpreted_by_shell: Some(true),
        }),
    };
    dap_server::write_server(req);
    let mut additional_data = serde_json::Map::new();
    additional_data.insert(
        "include".to_string(),
        serde_json::Value::String("./linux/**/*.c".to_string()),
    );
    additional_data.insert(
        "log_file".to_string(),
        serde_json::Value::String("./linux.log".to_string()),
    );
    additional_data.insert(
        "log_pattern".to_string(),
        serde_json::Value::String("\\[([^:]+):(?P<line>\\d+)\\] (?P<message>.*)$".to_string()),
    );
    let req = Request {
        seq: 2,
        command: Launch(LaunchRequestArguments {
            no_debug: None,
            restart_data: None,
            additional_data: Some(serde_json::Value::Object(additional_data)),
        }),
    };

    dap_server::write_server(req);
    let req = Request {
        seq: 3,
        command: SetBreakpoints(SetBreakpointsArguments {
            source: dap::types::Source {
                name: Some("linux.log".to_string()),
                path: Some("./linux.log".to_string()),
                source_reference: None,
                presentation_hint: None,
                origin: None,
                sources: None,
                adapter_data: None,
                checksums: None,
            },
            breakpoints: Some(vec![SourceBreakpoint {
                line: 10,
                column: None,
                condition: None,
                hit_condition: None,
                log_message: None,
            }]),
            lines: None,
            source_modified: Some(false),
        }),
    };
    dap_server::write_server(req);
    let req = Request {
        seq: 4,
        command: Continue(ContinueArguments {
            thread_id: 0,
            single_thread: None,
        }),
    };
    loop {
        app.app_loop();
        if dap_server::read_server().is_none() {
            break;
        }
    }

    dap_server::write_server(req);

    let start = Instant::now();

    loop {
        app.app_loop();
        if let Some(Sendable::Event(Event::Stopped(stop_event))) = dap_server::read_server() {
            println!("Got: {:?}", stop_event.description);
            break;
        }
    }

    let end = start.elapsed();
    println!("Time taken: {:.2}s", end.as_secs_f32());
}
