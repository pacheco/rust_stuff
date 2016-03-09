#![allow(unused_imports)]
#![allow(dead_code)]
#![allow(unused_variables)]
extern crate rust_stuff;
extern crate time;

use rust_stuff::RBTree;
use rust_stuff::BTree;
use std::collections::BTreeMap;
use std::io;
use std::io::Read;

const N:i32 = 10_000_000;

fn do_print_duration<F>(mut f: F) where F: FnMut() {
    let start = time::PreciseTime::now();
    f();
    let duration = start.to(time::PreciseTime::now());
    println!("operation took {}.{}s", duration.num_seconds(), duration.num_milliseconds() % 1000);
}

fn test_tree() {
    let mut t = BTree::new_with_order(11);
    // let mut t = RBTree::new();
    // let mut t = BTreeMap::new();

    do_print_duration(|| {
        for n in (1..N).rev() {
            t.insert(n, n*2);
        }
    });

    // println!("height: {}", t.height());

    // println!("press enter...");
    // io::stdin().read_line(&mut String::new());

    do_print_duration(|| {
        let mut foo = 0;
        for n in 1..N {
            let x = t.get(&n);
            foo += *x.unwrap();
            foo -= *x.unwrap();
        }
    });

    // let start = time::PreciseTime::now();
    // let mut items = t.iter();
    // let mut foo = 0;
    // for n in 1..N {
    //     let x = items.next();
    //     foo += *x.unwrap().1;
    // }
    // let duration = start.to(time::PreciseTime::now());
    // println!("operation took {}.{}s", duration.num_seconds(), duration.num_milliseconds() % 1000);

    // let start = time::PreciseTime::now();
    // for n in 1..N {
    //     match t.remove(&n) {
    //         Some(i) => assert_eq!(i, n*2),
    //         _ => panic!(n),
    //     }
    // }
    // let duration = start.to(time::PreciseTime::now());
    // println!("operation took {}.{}s", duration.num_seconds(), duration.num_milliseconds() % 1000);
}

fn main() {
    test_tree();
}
