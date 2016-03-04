extern crate btree;

use btree::BTree;
use std::fmt::Debug;

fn insert<K: Ord + Debug, V: Debug>(t: &mut BTree<K,V>, k: K, v: V) {
    match t.insert(k, v) {
        Some(v) => println!("Old value was: {:?}", v),
        _ => {}
    }
    //t.breath_first_print();
}

fn main() {
    let mut r: BTree<i32, i32> = BTree::new(4); //
    for n in 1..1000 {
        insert(&mut r, n, n);
    }

    for kv in r.into_iter() {
        println!("{:?}", kv);
    }
}
