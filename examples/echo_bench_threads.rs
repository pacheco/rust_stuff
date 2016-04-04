extern crate time;
extern crate rust_stuff;

use rust_stuff::net::FramedTcpStream;
use std::net::TcpStream;
use std::thread;
use std::sync::{Arc, Mutex};
use std::env;

const ADDR: &'static str = "127.0.0.1:10000";
const SIZE: usize = 32;

#[derive(Default)]
struct Counters {
    count: u64,
    lat: u64,
    max_lat: u64,
}

fn main() {
    println!("Sending messages of size {}", SIZE);
    let counters = Arc::new(Mutex::new(Counters::default()));
    let mut threads = vec!();

    let n_threads = env::args().nth(1).unwrap().parse::<u32>().unwrap();

    for n in 0..n_threads {
        let counters = counters.clone();
        threads.push(thread::spawn(move || {
            let mut stream = match TcpStream::connect(&ADDR[..]) {
                Ok(stream) => FramedTcpStream::new(stream),
                Err(e) => panic!(e),
            };
            println!("Thread {} connected!", n);
            let mut msg = Vec::with_capacity(SIZE);
            msg.resize(SIZE, 1);
            let mut buf: [u8; SIZE] = [0; SIZE];
            loop {
                // send msg
                let sendtime = time::PreciseTime::now();
                stream.write_frame(msg.as_slice()).unwrap();
                stream.read_frame_into(&mut buf).unwrap();
                let mut c = counters.lock().unwrap();
                c.count += 1;
                let now = time::PreciseTime::now();
                let l = sendtime.to(now).num_microseconds().unwrap() as u64;
                if c.lat == 0 {
                    c.lat = l;
                    c.max_lat = l;
                } else {
                    c.lat += l;
                    c.lat = c.lat/2;
                    c.max_lat = if l > c.max_lat { l } else { c.max_lat }
                }
            }
        }));
    }

    let mut start = time::PreciseTime::now();
    loop {
        thread::sleep(std::time::Duration::from_secs(1));
        let now = time::PreciseTime::now();
        let duration = start.to(now);
        start = now;
        let mut c = counters.lock().unwrap();
        println!("tput: {} op/sec\tavg_lat: {} usec\tmax_lat: {}", 
                 (c.count*1000) as f64 / duration.num_milliseconds() as f64,
                 c.lat,
                 c.max_lat);
        c.count = 0;
        c.lat = 0;
    }
}
