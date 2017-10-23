use std::thread;
use std::sync::{Arc,Mutex};
use std::time::{Instant,Duration};

fn work(id: i32, counter: Arc<Mutex<u64>>) {
    let mut start = Instant::now();
    let onesec = Duration::from_secs(1);
    let mut local_counter = 0u64;
    loop {
        let mut counter = counter.lock().unwrap();
        *counter += 1;
        local_counter += 1;
        let now = Instant::now();
        if now.duration_since(start) > onesec {
            println!("t{}: {}", id, local_counter);
            local_counter = 0;
            start = now;
        }
    }
}

fn main() {
    let counter = Arc::new(Mutex::new(0u64));
    for i in 1..10 {
        let counter = counter.clone();
        thread::spawn(move || {
            work(i, counter);
        });
    }
    work(0, counter.clone());
}
