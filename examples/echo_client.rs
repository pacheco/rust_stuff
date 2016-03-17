extern crate rust_stuff;

use rust_stuff::net::FramedTcpStream;
use std::net::TcpStream;
use std::io;

const ADDR: &'static str = "127.0.0.1:10000";

fn main() {
    let mut stream = FramedTcpStream::new(TcpStream::connect(&ADDR[..]).unwrap());
    let mut msg = String::new();
    while let Ok(_) = io::stdin().read_line(&mut msg) {
        // send msg
        stream.write_frame(msg.trim().as_bytes()).unwrap();
        msg.clear();
        let msg = stream.next().unwrap();
        println!("reply: {}", String::from_utf8(msg).unwrap());
    }
}
