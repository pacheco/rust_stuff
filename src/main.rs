extern crate btree;
extern crate rand;

use btree::BTree;
use std::collections::BTreeMap;
use std::io;
use std::io::Read;

const N:i32 = 10000000;

fn main() {
    // let mut t: BTree<i32, i32> = BTree::new(10);
    // for n in (1..N).rev() {
    //     t.insert(n, n*2);
    // }

    // println!("height: {}", t.height());
    // println!("len: {}", t.len());

    let mut t: BTreeMap<i32, i32> = BTreeMap::new(); //
    for n in (1..N).rev() {
        t.insert(n, n*2);
    }

    // println!("press enter...");
    // io::stdin().read_line(&mut String::new());

    // for n in 1..N {
    //     t.get(&n);
    // }

    // let mut items = t.iter();
    // for n in 1..N {
    //     items.next();
    // }

    //t.breath_first_debug_print(false);

    for n in 1..N {
        match t.remove(&n) {
            Some(i) => assert_eq!(i, n*2),
            _ => panic!(n),
        }
    }

    // for kv in t.into_iter() {
    //     println!("{:?}", kv);
    // }
}
