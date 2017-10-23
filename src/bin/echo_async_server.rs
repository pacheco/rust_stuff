extern crate rust_stuff;
#[macro_use]
extern crate log;
extern crate env_logger;

use std::env;
use std::collections::HashSet;
use std::thread;
use rust_stuff::net::async::{ConnectionUid, Server, ServerHandler, ServerControl, Error};

struct MyHandler {
    connections: HashSet<ConnectionUid>,
}

impl ServerHandler for MyHandler {
    type Message = String;
    type Timeout = String;
    fn init(&mut self, server: &mut ServerControl<Self>) {
        info!("init");

        // testing channel notify
        let chan = server.notify_channel();
        thread::spawn(move || {
            thread::sleep(std::time::Duration::from_secs(5));
            chan.send(String::from("hello from other thread!")).unwrap();
        });

        // testing timeout
        server.timeout_ms(String::from("async timeout!"), 2000u64).unwrap();
    }
    fn connection(&mut self, _server: &mut ServerControl<Self>, uid: ConnectionUid) {
        self.connections.insert(uid);
        info!("new connection");
    }
    fn connection_closed(&mut self, _server: &mut ServerControl<Self>, uid: &ConnectionUid) {
        self.connections.remove(uid);
        info!("disconnect");
    }
    fn message(&mut self, server: &mut ServerControl<Self>, uid: &ConnectionUid, msg: Vec<u8>){
        debug!("handler message called");
        // println!("msg: {}", String::from_utf8(msg.clone()).unwrap());
        server.send(uid, &msg[..]);
    }
    fn notify(&mut self, _server: &mut ServerControl<Self>, msg: Self::Message) {
        info!("notify msg: {}", msg);
    }
    fn timeout(&mut self, _server: &mut ServerControl<Self>, timeout: Self::Timeout) {
        info!("timeout: {}", timeout);
    }
    fn shutting_down(&mut self, _err: Option<Error>) {
        info!("shutting down...");
    }
}

fn main() {
    if env::var("RUST_LOG").is_ok() {
        env_logger::init().unwrap();
    }
    else {
        env_logger::LogBuilder::new().parse("info").init().unwrap();
    }
    let addr = "127.0.0.1:10000".parse().unwrap();
    let mut server = Server::bind(&addr, MyHandler{ connections: HashSet::new() }, 128).unwrap();
    server.run().unwrap();
}
