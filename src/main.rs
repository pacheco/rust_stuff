extern crate btree;
extern crate rand;

use btree::BTree;
use std::collections::BTreeMap;
use std::io;
use std::io::Read;

const N:i32 = 10000000;

fn main() {
    let mut t: BTree<i32, i32> = BTree::new(10); //
    for n in 1..N {
        t.insert(n, n*2);
    }

    // let mut t: BTreeMap<i32, i32> = BTreeMap::new(); //
    // for n in 1..N {
    //     t.insert(n, n*2);
    // }

    println!("press enter...");
    io::stdin().read_line(&mut String::new());

    // for n in 1..N {
    //     t.get(&n);
    // }

    // let mut items = t.iter();
    // for n in 1..N {
    //     items.next();
    // }

    // t.breath_first_print();

    // println!("{:?}", t.get(&3));

    // for kv in t.into_iter() {
    //     println!("{:?}", kv);
    // }
}
