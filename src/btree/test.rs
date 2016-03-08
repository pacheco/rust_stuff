use btree::BTree;
use btree::Node;
use std::io::{stdout,Write};
use std::fmt::Debug;

impl<K, V> BTree<K, V>  where K: Ord + Debug, V: Debug {
    /// Print keys in breath first order. Same level keys are printed on the same line
    pub fn breath_first_debug_print(&self, print_values: bool) {
        let mut nodes: Vec<&Box<Node<K,V>>> = vec![];
        let mut height = 0;
        let mut height_nodes = 1; // tracks how many nodes we still need to pop in this height
        let mut next_height_nodes = 0; // accumulator for the number of nodes on the next height
        nodes.insert(0, &self.root);
        while !nodes.is_empty() {
            let n = nodes.pop().unwrap();
            height_nodes -= 1;
            next_height_nodes += n.children.len();
            if print_values {
                print!("{:?}=>{:?} ", n.keys, n.values);
            } else {
                print!("{:?} ", n.keys);
            }
            if height_nodes == 0 {
                // finished printing this height
                height += 1;
                height_nodes = next_height_nodes;
                next_height_nodes = 0;
                println!("");
            }
            for c in &n.children {
                nodes.insert(0, c);
            }
        }
        stdout().flush().unwrap();
        println!("Tree has height {}", height);
    }
}

#[test]
fn into_iter_test() {
    let mut r: BTree<i32, i32> = BTree::new();
    for n in 1..1000 {
        r.insert(n, 2*n);
    }

    let mut r = r.into_iter();
    for n in 1..1000 {
        let (k,v) = r.next().unwrap();
        assert_eq!(k, n);
        assert_eq!(v, 2*n);
    }

    let mut r: BTree<i32, i32> = BTree::new();
    for n in (1..1000).rev() {
        r.insert(n, 2*n);
    }

    let mut r = r.into_iter();
    for n in 1..1000 {
        let (k,v) = r.next().unwrap();
        assert_eq!(k, n);
        assert_eq!(v, 2*n);
    }
}

#[test]
fn iter_test() {
    let mut r: BTree<i32, i32> = BTree::new();
    for n in 1..1000 {
        r.insert(n, 2*n);
    }

    let mut r = r.iter();
    for n in 1..10 {
        let (k,v) = r.next().unwrap();
        assert_eq!(*k, n);
        assert_eq!(*v, 2*n);
    }

    let mut r: BTree<i32, i32> = BTree::new();
    for n in (1..1000).rev() {
        r.insert(n, 2*n);
    }

    let mut r = r.iter();
    for n in 1..10 {
        let (k,v) = r.next().unwrap();
        assert_eq!(*k, n);
        assert_eq!(*v, 2*n);
    }
}

#[test]
fn get_test() {
    let mut r: BTree<i32, i32> = BTree::new();
    for n in 1..1000 {
        r.insert(n, 2*n);
    }

    for n in 1..1000 {
        match r.get(&n) {
            Some(i) => assert_eq!(*i, n*2),
            _ => panic!(),
        }
    }

    assert_eq!(r.get(&0), None);

    let mut r: BTree<i32, i32> = BTree::new();
    for n in (1..1000).rev() {
        r.insert(n, 2*n);
    }

    for n in 1..1000 {
        match r.get(&n) {
            Some(i) => assert_eq!(*i, n*2),
            _ => panic!(),
        }
    }

    assert_eq!(r.get(&0), None);
}

#[test]
fn test_remove() {
    let mut r: BTree<i32, i32> = BTree::new();
    for n in 1..1000 {
        r.insert(n, 2*n);
    }

    for n in 1..1000 {
        match r.remove(&n) {
            Some(i) => assert_eq!(i, n*2),
            _ => panic!(n),
        }
    }

    let mut r: BTree<i32, i32> = BTree::new();
    for n in (1..1000).rev() {
        r.insert(n, 2*n);
    }

    for n in 1..1000 {
        match r.remove(&n) {
            Some(i) => assert_eq!(i, n*2),
            _ => panic!(n),
        }
    }
}

#[test]
fn test_order() {
    let r: BTree<i32, i32> = BTree::new_with_order(3);
    assert_eq!(r.m, 4);
}
