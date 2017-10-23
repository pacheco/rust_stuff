extern crate chan;

use std::thread;
use std::time::{Instant,Duration};

fn receive(receiver: chan::Receiver<bool>) {
    let mut start = Instant::now();
    let onesec = Duration::from_secs(1);
    let mut local_counter = 0u64;
    loop {
        receiver.recv();
        local_counter += 1;
        let now = Instant::now();
        if now.duration_since(start) > onesec {
            println!("{}", local_counter);
            local_counter = 0;
            start = now;
        }
    }
}

fn main() {
    let n = 5;
    let (sender, receiver) = chan::sync(n*2);
    for _ in 0..n {
        let sender = sender.clone();
        thread::spawn(move || {
            loop {
                sender.send(true);
                //thread::sleep_ms(100);
            }
        });
    }
    receive(receiver);
}
