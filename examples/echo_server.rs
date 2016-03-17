extern crate rust_stuff;

use std::net::TcpListener;
use rust_stuff::net::FramedTcpStream;
use std::thread;

const ADDR: &'static str = "127.0.0.1:10000";

fn handle_client(mut stream: FramedTcpStream) {
    loop {
        let msg = stream.next();
        stream.write_frame(msg.unwrap().as_slice()).unwrap();
    }
}

fn main() {
    let listener = TcpListener::bind(&ADDR[..]).unwrap();
    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                let framed = FramedTcpStream::new(stream);
                thread::spawn(move || handle_client(framed));
            }
            Err(err) => {
                panic!(err);
            }
        }
    }
}
