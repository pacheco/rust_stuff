extern crate rust_stuff;

use std::net::TcpListener;
use rust_stuff::net::FramedTcpStream;
use std::thread;

const MAX_SIZE: usize = 32*1024;
const ADDR: &'static str = "127.0.0.1:10000";

fn handle_client(mut stream: FramedTcpStream) {
    let mut buf: [u8;MAX_SIZE] = [0;MAX_SIZE];
    loop {
        let len = stream.read_frame_into(&mut buf).unwrap();
        stream.write_frame(&buf[0..len]).unwrap();
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
