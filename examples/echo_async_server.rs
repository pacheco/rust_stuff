extern crate rust_stuff;
#[macro_use]
extern crate log;
extern crate env_logger;

use std::collections::HashSet;
use rust_stuff::net::async::{ConnectionUid, Server, ServerHandler, ServerControl, Error};

struct MyHandler {
    connections: HashSet<ConnectionUid>,
}

impl ServerHandler for MyHandler {
    type Message = ();
    type Timeout = ();
    fn connection(&mut self, _server: &mut ServerControl, uid: ConnectionUid) {
        self.connections.insert(uid);
        debug!("handler new connection");
    }
    fn connection_closed(&mut self, _server: &mut ServerControl, uid: &ConnectionUid) {
        self.connections.remove(uid);
        debug!("handler disconnect");
    }
    fn message(&mut self, server: &mut ServerControl, uid: &ConnectionUid, msg: Vec<u8>){
        debug!("handler message called");
        server.send(uid, &msg[..]);
    }
    fn shutting_down(&mut self, _err: Option<Error>) {
        debug!("shutting down...");
    }
}

fn main() {
    env_logger::init().unwrap();
    let addr = "127.0.0.1:10000".parse().unwrap();
    let mut server = Server::bind(&addr, MyHandler{ connections: HashSet::new() }).unwrap();
    server.run().unwrap();
}
