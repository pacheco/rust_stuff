extern crate btree;

use btree::BTree;
use std::fmt::Debug;

fn insert<K: Ord + Debug, V: Debug>(t: &mut BTree<K,V>, k: K, v: V) {
    match t.insert(k, v) {
        Some(v) => println!("Old value was: {:?}", v),
        _ => {}
    }
    // println!("{:?}", t);
}

fn main() {
    let mut r: BTree<i32, &str> = BTree::new(5); //
    insert(&mut r, 1, "a");
    insert(&mut r, 2, "b");
    insert(&mut r, 3, "c");
    insert(&mut r, 4, "d");
    insert(&mut r, 5, "e");
    insert(&mut r, 6, "f");
    insert(&mut r, 7, "g");
    insert(&mut r, 8, "h");

    println!("");
    println!("");
    println!("");

    r.breath_first_print();

    r.depth_first_print();
}
