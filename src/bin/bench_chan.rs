use std::thread;
use std::sync::mpsc;
use std::time::{Instant,Duration};

fn receive(receiver: mpsc::Receiver<bool>) {
    let mut start = Instant::now();
    let onesec = Duration::from_secs(1);
    let mut local_counter = 0u64;
    loop {
        receiver.recv().unwrap();
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
    let (sender, receiver) = mpsc::sync_channel(n*2);
    for _ in 0..n {
        let sender = sender.clone();
        thread::spawn(move || {
            loop {
                sender.send(true).unwrap();
                //thread::sleep_ms(100);
            }
        });
    }
    receive(receiver);
}
