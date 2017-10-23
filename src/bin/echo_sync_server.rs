extern crate rust_stuff;

use std::net::{SocketAddr};
use rust_stuff::net::sync::{Server,Event};

const ADDR: &'static str = "127.0.0.1:10000";

fn main() {
    let mut server = Server::new(ADDR.parse::<SocketAddr>().unwrap());
    assert!(server.start().is_ok());
    println!("listening...");
    loop {
        let evt = server.next().unwrap();
        //println!("{:?}", evt);
        if let Event::Recv(uid, data) = evt {
            server.send_to(uid, data.as_slice()).is_ok();
        }
    }
}
