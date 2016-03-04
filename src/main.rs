extern crate btree;
extern crate rand;

use btree::BTree;
use std::collections::BTreeMap;

fn main() {
    let mut t: BTree<i32, i32> = BTree::new(10); //
    for n in 1..1000000 {
        t.insert(n, n*2);
    }

    // let mut t: BTreeMap<i32, i32> = BTreeMap::new(); //
    // for n in 1..1000000 {
    //     t.insert(n, n*2);
    // }

    for n in 1..1000000 {
        t.get(&n);
    }

    let mut items = t.iter();
    for n in 1..1000000 {
        items.next();
    }

    // t.breath_first_print();

    // println!("{:?}", t.get(&3));

    // for kv in t.into_iter() {
    //     println!("{:?}", kv);
    // }
}
