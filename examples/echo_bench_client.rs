extern crate time;
extern crate rust_stuff;

use rust_stuff::net::FramedTcpStream;
use std::net::TcpStream;

const ADDR: &'static str = "127.0.0.1:10000";
const SIZE: usize = 32;

fn main() {
    let mut stream = FramedTcpStream::new(TcpStream::connect(&ADDR[..]).unwrap());
    let mut msg = Vec::with_capacity(SIZE);
    msg.resize(SIZE, 1);
    let mut count: u64 = 0;
    let mut lat: u64 = 0;
    let mut start = time::PreciseTime::now();
    let mut buf: [u8; SIZE] = [0; SIZE];
    loop {
        // send msg
        let sendtime = time::PreciseTime::now();
        stream.write_frame(msg.as_slice()).unwrap();
        msg.clear();
        stream.read_frame_into(&mut buf).unwrap();
        count += 1;
        let now = time::PreciseTime::now();
        if lat == 0 {
            lat = sendtime.to(now).num_microseconds().unwrap() as u64;
        } else {
            lat += sendtime.to(now).num_microseconds().unwrap() as u64;
            lat = lat/2;
        }
        let duration = start.to(now);
        if duration.num_seconds() > 0 {
            start = time::PreciseTime::now();
            println!("tput: {} avg_lat: {}", (count*1000) as f64 / duration.num_milliseconds() as f64, lat);
            count = 0;
            lat = 0;
        }
    }
}
