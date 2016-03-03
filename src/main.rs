extern crate btree;

use btree::BTree;

fn insert_print<K: Ord + Clone,V: Clone>(t: &mut BTree<K,V>, k: K, v: V) {
}

fn main() {
    let mut r: BTree<i32,String> = BTree::new(1);
    println!("{:?}", r.insert(1, String::from("burzum")));
    println!("{:?}", r);
}
