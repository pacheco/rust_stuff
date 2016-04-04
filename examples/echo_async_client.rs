extern crate rust_stuff;
#[macro_use]
extern crate log;
extern crate env_logger;

use std::env;
use std::collections::HashSet;
use std::thread;
use std::io;
use rust_stuff::net::async::{ConnectionUid, Server, ServerHandler, ServerControl, Error, Sender};

struct MyHandler {
    connections: HashSet<ConnectionUid>,
}

fn read_msg(uid: ConnectionUid, chan: Sender<(ConnectionUid, String)>) {
    let mut msg = String::new();
    if let Ok(_) = io::stdin().read_line(&mut msg) {
        // send msg
        chan.send((uid, String::from(msg.trim()))).unwrap();
    } else {
        chan.send((uid, String::from(""))).unwrap();
    }
}

impl ServerHandler for MyHandler {
    type Message = (ConnectionUid, String);
    type Timeout = String;
    fn init(&mut self, server: &mut ServerControl<Self>) {
        info!("init");
        let uid = server.connect("127.0.0.1:10000".parse().unwrap()).unwrap();
        // testing channel notify
        let chan = server.notify_channel().clone();
        thread::spawn(move || { read_msg(uid.clone(), chan); });
    }
    fn connection(&mut self, _server: &mut ServerControl<Self>, uid: ConnectionUid) {
        self.connections.insert(uid);
        info!("connected!");
    }
    fn connection_closed(&mut self, server: &mut ServerControl<Self>, uid: &ConnectionUid) {
        self.connections.remove(uid);
        info!("disconnected");
        server.shutdown();
    }
    fn message(&mut self, server: &mut ServerControl<Self>, uid: &ConnectionUid, msg: Vec<u8>){
        println!("got reply: {}", String::from_utf8(msg).unwrap());
        // testing channel notify
        let chan = server.notify_channel().clone();
        let uid = uid.clone();
        thread::spawn(move || { read_msg(uid, chan); });
    }
    fn notify(&mut self, server: &mut ServerControl<Self>, msg: Self::Message) {
        if !msg.1.is_empty() {
            server.send(&msg.0, msg.1.as_bytes());
        } else {
            server.shutdown();
        }
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
    let mut server = Server::new(MyHandler{ connections: HashSet::new() }, 1).unwrap();
    server.run().unwrap();
}
