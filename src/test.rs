pub mod app_state;
mod dap_logger;
mod dap_server;
mod log_search;

use std::collections::HashMap;
use std::fs;
use std::time::Instant;

use dap::requests::Command::*;
use dap::requests::*;
use regex::Regex;
use serde_json::Map;

use dap::base_message::Sendable;
use dap::events::*;
use dap::types::*;
use serde_json::Value;

use crate::app_state::*;

#[macro_use]
extern crate log;

fn main() {
    test_breakpoints_10_lines();
    test_breakpoints_10_lines_and_func();
    test_breakpoints_in_source_lines();
}

fn gen_init(seq: i64) -> Request {
    Request {
        seq,
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
    }
}

fn gen_launch(seq: i64, additional_data: Map<String, Value>) -> Request {
    Request {
        seq,
        command: Launch(LaunchRequestArguments {
            no_debug: None,
            restart_data: None,
            additional_data: Some(serde_json::Value::Object(additional_data)),
        }),
    }
}

struct Breakpoint {
    pub path: String,
    pub line: i64,
}

fn gen_breakpoints(seq: i64, breakpoints: &Vec<Breakpoint>) -> Vec<Request> {
    let mut seq = seq;
    let mut breakpoint_map: HashMap<String, Vec<i64>> = HashMap::new();
    for b in breakpoints.iter() {
        match breakpoint_map.entry(b.path.clone()) {
            std::collections::hash_map::Entry::Occupied(mut o) => {
                o.get_mut().push(b.line);
            }
            std::collections::hash_map::Entry::Vacant(v) => {
                v.insert(vec![b.line]);
            }
        }
    }

    breakpoint_map
        .iter()
        .map(|(p, b)| {
            seq += 1;
            Request {
                seq: seq - 1,
                command: SetBreakpoints(SetBreakpointsArguments {
                    source: dap::types::Source {
                        name: Some(p.clone()),
                        path: Some(p.clone()),
                        source_reference: None,
                        presentation_hint: None,
                        origin: None,
                        sources: None,
                        adapter_data: None,
                        checksums: None,
                    },
                    breakpoints: Some(
                        b.iter()
                            .map(|v| SourceBreakpoint {
                                line: *v,
                                column: None,
                                condition: None,
                                hit_condition: None,
                                log_message: None,
                            })
                            .collect(),
                    ),
                    lines: None,
                    source_modified: Some(false),
                }),
            }
        })
        .collect()
}

fn gen_continue() -> Request {
    Request {
        seq: 4,
        command: Continue(ContinueArguments {
            thread_id: 0,
            single_thread: None,
        }),
    }
}

fn run_test(additional_data: Map<String, Value>, breakpoints: &Vec<Breakpoint>) {
    let mut app = App::init();
    dap_server::write_server(gen_init(1));

    dap_server::write_server(gen_launch(2, additional_data));
    for ele in gen_breakpoints(3, &breakpoints) {
        dap_server::write_server(ele);
    }
    loop {
        app.app_loop();
        if dap_server::read_server().is_none() {
            break;
        }
    }

    dap_server::write_server(gen_continue());

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

fn test_breakpoints_10_lines_and_func() {
    println!("\n\nRunning test: 10 lines until breakpoint. Filename+Line enabled.");
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

    let breakpoints = vec![Breakpoint {
        path: "./linux.log".to_string(),
        line: 30,
    }];
    run_test(additional_data, &breakpoints);
}

fn test_breakpoints_10_lines() {
    println!("\n\nRunning test: 10 lines until breakpoint. Line numbers enabled only");
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

    let breakpoints = vec![Breakpoint {
        path: "./linux.log".to_string(),
        line: 30,
    }];
    run_test(additional_data, &breakpoints);
}

fn test_breakpoints_in_source_lines() {
    println!("\n\nRunning test: 10 lines until breakpoint (in source). Line numbers enabled only");
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

    // Find line 10 in the source
    let contents = std::fs::read_to_string("./linux.log").unwrap();
    let line = contents.lines().nth(30 - 1).unwrap();
    let regex = Regex::new(r"\[(?P<file>[^:]+):(?P<line>\d+)\] (?P<message>.*)$").unwrap();
    let line_number = regex
        .captures(line)
        .unwrap()
        .name("line")
        .unwrap()
        .as_str()
        .parse()
        .unwrap();
    let file_name = regex.captures(line).unwrap().name("file").unwrap().as_str();

    let breakpoints = vec![Breakpoint {
        path: file_name.to_string(),
        line: line_number,
    }];
    run_test(additional_data, &breakpoints);
}
