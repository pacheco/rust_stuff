extern crate time;
extern crate rust_stuff;
extern crate byteorder;

use rust_stuff::net::FramedTcpStream;
use std::net::TcpStream;
use std::thread;
use std::sync::{Arc, Mutex};
use std::env;
use std::ops::Add;
use byteorder::{BigEndian,ByteOrder};

const ADDR: &'static str = "127.0.0.1:10000";
const SIZE: usize = 32;

#[derive(Default)]
struct Counters {
    lat: Vec<u64>,
}

fn main() {
    println!("Sending messages of size {}", SIZE);
    let counters = Arc::new(Mutex::new(Counters::default()));
    let mut threads = vec!();

    let n_threads = env::args().nth(1).unwrap().parse::<u32>().unwrap();

    let addr =
        if env::args().len() == 3 {
            env::args().nth(2).unwrap()
        } else {
            ADDR.to_string()
        };

    for n in 0..n_threads {
        let counters = counters.clone();
        let addr = addr.clone();
        threads.push(thread::spawn(move || {
            let mut stream = match TcpStream::connect(addr) {
                Ok(stream) => FramedTcpStream::new(stream),
                Err(e) => panic!(e),
            };
            println!("Thread {} connected!", n);

            let msg = "hello world! hello world! hello!".as_bytes();
            let mut frame = Vec::with_capacity(4+msg.len());
            unsafe { frame.set_len(4) };
            BigEndian::write_u32(&mut frame[..4], msg.len() as u32);
            frame.extend_from_slice(msg);

            let mut buf: [u8; SIZE] = [0; SIZE];
            loop {
                // send msg
                let sendtime = time::PreciseTime::now();
                unsafe { stream.raw_write(&frame[..]).unwrap(); }
                stream.read_frame_into(&mut buf).unwrap();
                let mut c = counters.lock().unwrap();
                let now = time::PreciseTime::now();
                let l = sendtime.to(now).num_microseconds().unwrap() as u64;
                c.lat.push(l);
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
        let max_lat = c.lat.iter().cloned().fold(0, std::cmp::max);
        let count = c.lat.len();
        let avg_lat = c.lat.iter().cloned().fold(0, u64::add) as f64 / count as f64;
        let square_diff = |x| {(x-avg_lat)*(x-avg_lat)};
        let std_dev = (c.lat.iter().cloned()
                       .map(|x| x as f64)
                       .map(square_diff)
                       .fold(0f64, f64::add) / count as f64).sqrt();
        c.lat.sort();
        let median = c.lat[count/2];
        let perc95 = c.lat[(count * 95) / 100];
        let perc99 = c.lat[(count * 99) / 100];
        let perc999 = c.lat[(count * 999) / 1000];
        // println!("{} {} {} {} {}",
        //          count/2,
        //          (count * 95) / 100,
        //          (count * 99) / 100,
        //          (count * 999) / 1000,
        //          count);
        c.lat = Vec::new();
        println!("tput: {} op/sec\tavg_lat: {} usec\tmax_lat: {}\tstd_dev: {}\n\
                  \tmedian: {}\t95th: {}\t99th: {}\t99.9th: {}",
                 (count*1000) as f64 / duration.num_milliseconds() as f64,
                 avg_lat as u64,
                 max_lat,
                 std_dev as u64,
                 median,
                 perc95,
                 perc99,
                 perc999,
        );
    }
}
