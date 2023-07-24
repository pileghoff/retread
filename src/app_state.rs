use std::fs;
use std::path::PathBuf;

use dap::base_message::Sendable;
use dap::events::*;
use dap::requests::*;
use dap::responses::*;
use dap::types::*;
use glob::glob;
use regex::Regex;
use serde_json::Value;

use crate::{dap_server, log_search::*};

use anyhow::{anyhow, Context, Result};

#[derive(Clone, Debug)]
struct RetreadBreakpoint {
    path: String,
    line: i64,
}

impl RetreadBreakpoint {
    pub fn new(source: &Source, breakpoint: &SourceBreakpoint) -> Option<Self> {
        Some(RetreadBreakpoint {
            path: source.path.clone()?,
            line: breakpoint.line,
        })
    }
}

#[derive(Clone, Debug)]
pub enum AppState {
    Uninitialized(UninitializedState),
    Running(RunningState),
    Exit,
}

#[derive(Clone, Debug)]
pub struct UninitializedState {
    settings: Option<LogSearchSettings>,
}

impl UninitializedState {
    pub fn new() -> Self {
        UninitializedState { settings: None }
    }

    fn load_settings(additional_data: &Option<Value>) -> Result<LogSearchSettings> {
        if let Some(Value::Object(data)) = additional_data {
            let log_file = data
                .get("log_file")
                .context("Missing log file")?
                .as_str()
                .context("Log file is not a string")?;
            let regex = data
                .get("log_pattern")
                .context("Missing log_pattern")?
                .as_str()
                .context("log_pattern is not a valid string")?;
            let include_pattern: Result<Vec<_>> = data
                .get("include")
                .context("Missing include")?
                .as_array()
                .context("include is not a valid array")?
                .iter()
                .map(|v| {
                    v.as_str()
                        .context(format!("Include argument [{}] is not a valid string", v))
                        .and_then(|v| Ok(v.to_string()))
                })
                .collect();

            let exclude_pattern: Result<Vec<_>> = data
                .get("exclude")
                .context("Missing exclude")?
                .as_array()
                .context("include is not a valid array")?
                .iter()
                .map(|v| {
                    v.as_str()
                        .context(format!("Include argument [{}] is not a valid string", v))
                        .and_then(|v| Ok(v.to_string()))
                })
                .collect();

            return Ok(LogSearchSettings {
                log_file_name: log_file.to_string(),
                log_file: fs::read_to_string(log_file)?,
                log_pattern: Regex::new(regex)?,
                include: include_pattern?,
                exclude: exclude_pattern?,
            });
        }
        Err(anyhow!("Init message did not contain additional data"))
    }

    pub fn run(&mut self) -> Result<Option<AppState>> {
        let request = match dap_server::read() {
            Some(req) => req,
            None => return Ok(None),
        };
        info!("Got: {:?}", request);

        match request.command {
            Command::Initialize(_) => {
                let rsp = request.success(ResponseBody::Initialize(Capabilities {
                    supports_step_back: Some(true),
                    supports_restart_request: Some(false),
                    ..Default::default()
                }));

                dap_server::write(Sendable::Response(rsp));
            }

            Command::Launch(ref arguments) => {
                let resp = match UninitializedState::load_settings(&arguments.additional_data) {
                    Ok(s) => {
                        self.settings = Some(s);
                        dap_server::write(Sendable::Event(Event::Initialized));
                        request.ack()?
                    }
                    Err(e) => request.error(&e.to_string()),
                };
                dap_server::write(Sendable::Response(resp));

                if let Some(settings) = &self.settings {
                    let mut running_state = RunningState::new(settings.clone())?;
                    running_state.init()?;
                    return Ok(Some(AppState::Running(running_state)));
                }
            }

            _ => panic!("Invalid request"),
        }

        Ok(None)
    }
}

impl Default for UninitializedState {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Debug)]
pub struct RunningState {
    settings: LogSearchSettings,
    log_index: usize,
    breakpoints: Vec<RetreadBreakpoint>,
    running: bool,
    reverse: bool,
    files: Vec<(PathBuf, String)>,
}

impl RunningState {
    pub fn new(settings: LogSearchSettings) -> Result<Self> {
        let mut patterns = settings.include.clone();
        patterns.extend(settings.exclude.iter().map(|p| "!".to_string() + p));

        let include = settings
            .include
            .iter()
            .flat_map(|p| glob(p).unwrap())
            .flatten();
        let exclude: Vec<_> = settings
            .exclude
            .iter()
            .flat_map(|p| glob(p).unwrap())
            .flatten()
            .collect();

        let file_paths: Vec<_> = include.filter(|p| !exclude.contains(p)).collect();
        info!("Num paths: {}", file_paths.len());
        let files: Vec<_> = file_paths
            .iter()
            .flat_map(|f| match std::fs::read_to_string(f) {
                Ok(c) => Ok((f.clone(), c)),
                Err(e) => Err(anyhow!("{}", e)),
            })
            .collect();

        files.iter().for_each(|(f, c)| info!("{}", f.display()));

        Ok(RunningState {
            settings,
            log_index: 0,
            breakpoints: Vec::new(),
            running: false,
            reverse: false,
            files,
        })
    }

    pub fn init(&mut self) -> Result<()> {
        self.stop(StoppedEventReason::Entry)
    }

    fn get_log_line_search(&self) -> Result<LogLineSearch> {
        let log_line = self
            .settings
            .log_file
            .lines()
            .nth(self.log_index)
            .context(format!(
                "Unable to get line {} from log file",
                self.log_index
            ))?;
        LogLineSearch::new(&self.settings.log_pattern, log_line)
    }

    fn get_log_file_source(&self) -> Source {
        Source {
            name: Some(
                self.settings
                    .log_file_name
                    .split('/')
                    .last()
                    .unwrap()
                    .to_string(),
            ),
            path: Some(self.settings.log_file_name.to_string()),
            ..Default::default()
        }
    }

    fn clear_breakpoints_for_source(&mut self, source: &Source) {
        self.breakpoints.retain(|b| {
            if let Some(ref p) = source.path {
                *p != b.path
            } else {
                true
            }
        });
    }

    fn increment_log_index(&mut self) -> Result<()> {
        if self.reverse && self.log_index > 0 {
            self.log_index -= 1;
        } else if !self.reverse && self.log_index + 1 < self.settings.log_file.lines().count() {
            self.log_index += 1;
        } else if self.running {
            self.stop(StoppedEventReason::Entry)?;
        }

        Ok(())
    }

    fn get_log_match(&mut self) -> Result<LogMatch> {
        loop {
            let log_line = self
                .settings
                .log_file
                .lines()
                .nth(self.log_index)
                .context(format!(
                    "Unable to get line {} from log file",
                    self.log_index
                ))?;
            let res = search_files(&self.files, &self.settings.log_pattern, log_line);
            if let Some(res) = res {
                if res.score > 0 {
                    return Ok(res);
                }
            }

            self.increment_log_index()?;
        }
    }

    fn stop(&mut self, reason: StoppedEventReason) -> Result<()> {
        let stop_event = Event::Stopped(StoppedEventBody {
            reason,
            description: Some(self.get_log_line_search()?.message),
            thread_id: Some(0),
            preserve_focus_hint: Some(false),
            text: None,
            all_threads_stopped: Some(false),
            hit_breakpoint_ids: None,
        });

        dap_server::write(Sendable::Event(stop_event));
        self.running = false;

        Ok(())
    }

    pub fn run(&mut self) -> Result<Option<AppState>> {
        if self.running {
            self.increment_log_index()?;
            let m = self.get_log_match()?;
            if self.breakpoints.iter().any(|b| {
                (b.line == m.line as i64 && b.path == m.file)
                    || (b.line as usize == self.log_index + 1
                        && b.path == self.settings.log_file_name)
            }) {
                self.stop(StoppedEventReason::Breakpoint)?;
            }
        }
        let request = match dap_server::read() {
            Some(r) => r,
            None => return Ok(None),
        };
        info!("Got: {:?}", request);

        match request.command {
            Command::Next(_) | Command::StepIn(_) | Command::StepOut(_) => {
                dap_server::write(Sendable::Response(request.ack()?));
                self.stop(StoppedEventReason::Step)?;
                self.reverse = false;
                self.increment_log_index()?;
            }
            Command::ReverseContinue(_) => {
                dap_server::write(Sendable::Response(request.ack()?));
                self.reverse = true;
                self.running = true;
            }
            Command::StepBack(_) => {
                dap_server::write(Sendable::Response(request.ack()?));
                self.stop(StoppedEventReason::Step)?;
                self.reverse = true;
                self.increment_log_index()?;
            }
            Command::Pause(_) => {
                self.running = false;
                dap_server::write(Sendable::Response(request.ack()?));
                self.stop(StoppedEventReason::Pause)?;
            }
            Command::Continue(_) => {
                self.reverse = false;
                self.running = true;
                dap_server::write(Sendable::Response(request.success(ResponseBody::Continue(
                    ContinueResponse {
                        all_threads_continued: Some(true),
                    },
                ))));
            }

            Command::StackTrace(_) => {
                let log_match = self.get_log_match()?;
                let search_options = self.get_log_line_search()?;
                let name = log_match
                    .file
                    .split('/')
                    .last()
                    .context(format!("Unable to parse path {}", log_match.file))?
                    .to_string();
                let source = Source {
                    path: Some(log_match.file.clone()),
                    name: Some(name.clone()),
                    ..Default::default()
                };
                let frame_name = match search_options.func {
                    Some(func) => format!("{}:{}", func, log_match.line),
                    None => format!("{}:{}", name, log_match.line),
                };

                let frame = StackFrame {
                    id: 0,
                    name: frame_name,
                    source: Some(source),
                    line: log_match.line as i64,
                    ..Default::default()
                };

                let parent_frame = StackFrame {
                    id: 1,
                    name: self
                        .settings
                        .log_file_name
                        .split('/')
                        .last()
                        .context(format!(
                            "Unable to parse path {}",
                            self.settings.log_file_name
                        ))?
                        .to_string(),
                    source: Some(self.get_log_file_source()),
                    line: (self.log_index + 1) as i64,
                    ..Default::default()
                };

                dap_server::write(Sendable::Response(request.success(
                    ResponseBody::StackTrace(StackTraceResponse {
                        stack_frames: vec![frame, parent_frame],
                        total_frames: Some(1),
                    }),
                )));
            }
            Command::Threads => {
                dap_server::write(Sendable::Response(request.success(ResponseBody::Threads(
                    ThreadsResponse {
                        threads: vec![Thread {
                            id: 0,
                            name: "main".to_string(),
                        }],
                    },
                ))));
            }
            Command::Scopes(ref args) => {
                if args.frame_id == 0 {
                    let log_match = self.get_log_match()?;

                    let scope = Scope {
                        name: "Locals".to_string(),
                        presentation_hint: Some(ScopePresentationhint::Locals),
                        variables_reference: 133,
                        named_variables: None,
                        indexed_variables: None,
                        line: Some(log_match.line as i64),
                        ..Default::default()
                    };
                    dap_server::write(Sendable::Response(request.success(ResponseBody::Scopes(
                        ScopesResponse {
                            scopes: vec![scope],
                        },
                    ))));
                } else {
                    dap_server::write(Sendable::Response(
                        request
                            .success(ResponseBody::Scopes(ScopesResponse { scopes: Vec::new() })),
                    ));
                }
            }
            Command::Variables(_) => {
                let var = Variable {
                    name: "Variable name".to_string(),
                    value: self.get_log_line_search()?.message,
                    ..Default::default()
                };
                dap_server::write(Sendable::Response(request.success(
                    ResponseBody::Variables(VariablesResponse {
                        variables: vec![var],
                    }),
                )));
            }
            Command::SetBreakpoints(ref args) => {
                self.clear_breakpoints_for_source(&args.source);
                if let Some(new_breakpoints) = &args.breakpoints {
                    let breakpoints = new_breakpoints
                        .iter()
                        .filter_map(|b| RetreadBreakpoint::new(&args.source, b));

                    self.breakpoints.extend(breakpoints);
                }
            }
            Command::Disconnect(_) => {
                dap_server::write(Sendable::Response(request.ack()?));
                return Ok(Some(AppState::Exit));
            }

            Command::SetExceptionBreakpoints(_) => {}
            _ => error!("Unhandled request: {:?}", request),
        }
        Ok(None)
    }
}
pub struct App {
    pub state: AppState,
}

impl App {
    pub fn init() -> Self {
        App {
            state: AppState::Uninitialized(UninitializedState::new()),
        }
    }

    pub fn app_loop(&mut self) -> Result<()> {
        let res = match self.state {
            AppState::Uninitialized(ref mut s) => s.run()?,
            AppState::Running(ref mut s) => s.run()?,
            AppState::Exit => return Ok(()),
        };

        if let Some(s) = res {
            self.state = s;
        }

        Ok(())
    }
}
