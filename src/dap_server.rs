use crossbeam::channel::unbounded;
use crossbeam::channel::Receiver;
use crossbeam::channel::Sender;
use crossbeam::channel::TryRecvError;
use std::io::{stdin, stdout, BufReader, BufWriter, Stdin, Stdout};
use std::sync::Mutex;
use std::thread::spawn;

use dap::base_message::*;
use dap::requests::*;
use dap::responses::*;
use dap::server::*;

use lazy_static::lazy_static;

type StdServer = Server<Stdin, Stdout>;

lazy_static! {
    static ref SERVER: DapServer = DapServer::new();
}

#[cfg(not(feature = "test-server"))]
struct DapServer {
    outgoing: Mutex<StdServer>,
    incoming: Receiver<Request>,
}
#[cfg(feature = "test-server")]
struct DapServer {
    from_server: Receiver<Request>,
    to_server: Sender<Sendable>,
    to_client: Sender<Request>,
    from_client: Receiver<Sendable>,
}

impl DapServer {
    #[cfg(not(feature = "test-server"))]
    pub fn new() -> Self {
        let (tx, rx) = unbounded::<Request>();
        spawn(move || {
            let mut server = Server::new(BufReader::new(stdin()), BufWriter::new(stdout()));
            loop {
                let req = match server.poll_request() {
                    Ok(Some(req)) => req,
                    Ok(None) => continue,
                    Err(_) => return,
                };
                if tx.send(req).is_err() {
                    return;
                }
            }
        });

        let server = Server::new(BufReader::new(stdin()), BufWriter::new(stdout()));
        DapServer {
            outgoing: Mutex::new(server),
            incoming: rx,
        }
    }

    #[cfg(feature = "test-server")]
    pub fn new() -> Self {
        let (to_client, from_server) = unbounded::<Request>();
        let (to_server, from_client) = unbounded::<Sendable>();
        Self {
            to_client,
            from_client,
            to_server,
            from_server,
        }
    }
}

#[cfg(not(feature = "test-server"))]
pub fn read() -> Option<Request> {
    match SERVER.incoming.try_recv() {
        Ok(req) => Some(req),
        Err(TryRecvError::Disconnected) => None,
        Err(TryRecvError::Empty) => None,
    }
}

#[cfg(not(feature = "test-server"))]
pub fn write(message: Sendable) {
    SERVER.outgoing.lock().unwrap().send(message).unwrap();
}

#[cfg(feature = "test-server")]
pub fn read() -> Option<Request> {
    match SERVER.from_server.try_recv() {
        Ok(req) => Some(req),
        Err(TryRecvError::Disconnected) => None,
        Err(TryRecvError::Empty) => None,
    }
}

#[cfg(feature = "test-server")]
pub fn write(message: Sendable) {
    SERVER.to_server.send(message).unwrap();
}

#[cfg(feature = "test-server")]
pub fn read_server() -> Option<Sendable> {
    match SERVER.from_client.try_recv() {
        Ok(req) => Some(req),
        Err(TryRecvError::Disconnected) => None,
        Err(TryRecvError::Empty) => None,
    }
}

#[cfg(feature = "test-server")]
pub fn write_server(message: Request) {
    SERVER.to_client.send(message).unwrap();
}
