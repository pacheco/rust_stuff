extern crate btree;

use btree::BTree;
use std::fmt::Debug;

fn insert<K: Ord + Debug, V: Debug>(t: &mut BTree<K,V>, k: K, v: V) {
    match t.insert(k, v) {
        Some(v) => println!("Old value was: {:?}", v),
        _ => {}
    }
    println!("{:?}", t);
}

fn main() {
    let mut r: BTree<i32, &str> = BTree::new(1);
    insert(&mut r, 2, "foo");
    insert(&mut r, 1, "bar");
    insert(&mut r, 3, "baz");
}
