extern crate time;
extern crate rust_stuff;
extern crate byteorder;

use byteorder::BigEndian;
use byteorder::ByteOrder;

use rust_stuff::net::FramedTcpStream;
use std::net::TcpStream;

const ADDR: &'static str = "127.0.0.1:10000";
// const SIZE: usize = 32;

fn main() {
    let mut stream = FramedTcpStream::new(TcpStream::connect(&ADDR[..]).unwrap());

    // let mut msg = Vec::with_capacity(SIZE);
    // msg.resize(SIZE, 1);

    let msg = "hello world! hello world! hello!".as_bytes();
    let mut frame = Vec::with_capacity(4+msg.len());
    unsafe { frame.set_len(4) };
    BigEndian::write_u32(&mut frame[..4], msg.len() as u32);
    frame.extend_from_slice(msg);

    println!("Sending messages of size {}", msg.len());

    let mut count: u64 = 0;
    let mut lat: u64 = 0;
    let mut max_lat: u64 = 0;
    let mut start = time::PreciseTime::now();
    let mut buf: [u8; 1024] = [0; 1024];
    loop {
        // send msg
        let sendtime = time::PreciseTime::now();
        unsafe { stream.raw_write(&frame[..]).unwrap() };
        stream.read_frame_into(&mut buf).unwrap();
        count += 1;
        let now = time::PreciseTime::now();
        let l = sendtime.to(now).num_microseconds().unwrap() as u64;
        if lat == 0 {
            lat = l;
            max_lat = l;
        } else {
            lat += l;
            lat = lat/2;
            max_lat = if l > max_lat { l } else { max_lat }
        }
        let duration = start.to(now);
        if duration.num_seconds() > 0 {
            start = time::PreciseTime::now();
            println!("tput: {} op/sec\tavg_lat: {} usec\tmax_lat: {}", 
                     (count*1000) as f64 / duration.num_milliseconds() as f64,
                     lat,
                     max_lat);
            count = 0;
            lat = 0;
        }
    }
}
