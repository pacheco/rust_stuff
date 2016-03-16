extern crate rand;

use rbtree::{RBTree, BoxedNode};
use std::fmt::Debug;
use std::io::{stdout, Write};
use self::rand::{thread_rng, Rng};

#[test]
fn test_get() {
    let mut tree = RBTree::new();

    for i in (1..1000).rev() {
        tree.insert(i,i);
    }

    for i in 1..1000 {
        assert_eq!(tree.get(&i), Some(&i));
    }

    assert_eq!(tree.get(&1000), None);
    assert_eq!(tree.min(), Some(&1));
    assert_eq!(tree.max(), Some(&999));
}

#[test]
fn test_remove_min() {
    let mut tree = RBTree::new();

    for i in (1..1000).rev() {
        tree.insert(i,i);
    }
    for i in 1..1000 {
        assert_eq!(tree.root.remove_min().unwrap().value, Some(i));
    }
}


#[test]
fn test_remove() {
    let mut tree = RBTree::new();

    for i in (1..1000).rev() {
        tree.insert(i,i);
    }
    for i in 1..1000 {
        assert_eq!(tree.remove(&i), Some(i));
    }

    tree = RBTree::new();

    let mut rng = thread_rng();
    let mut shuffled = (1..1000).collect::<Vec<_>>();
    rng.shuffle(shuffled.as_mut_slice());
    for i in shuffled.into_iter() {
        tree.insert(i,i);
    }
    for i in 1..1000 {
        assert_eq!(tree.remove(&i), Some(i));
    }
}


#[allow(dead_code)]
fn debug_breadth_print<K,V>(tree: &RBTree<K,V>) where K: Ord + Debug {
    if tree.root.is_none() {
        println!("Empty tree");
        return;
    }
    let mut queue = vec![];
    queue.push(tree.root.as_ref().unwrap());
    let mut height = 0;
    let mut height_nodes = 1; // tracks how many nodes we still need to pop in this height
    let mut next_height_nodes = 0; // accumulator for the number of nodes on the next height
    while !queue.is_empty() {
        let n = queue.pop().unwrap();
        height_nodes -= 1;
        print!("{:?}:{:?} ", n.key, n.color);

        if n.left.is_some() {
            next_height_nodes += 1;
            queue.insert(0, n.left.as_ref().unwrap());
        }
        if n.right.is_some() {
            next_height_nodes += 1;
            queue.insert(0, n.right.as_ref().unwrap());
        }

        if height_nodes == 0 {
            // finished printing this height
            height += 1;
            height_nodes = next_height_nodes;
            next_height_nodes = 0;
            println!("");
        }
    }
    stdout().flush().unwrap();
    println!("Tree has height {}", height);
}

#[test]
#[ignore]
fn test_debug_breadth_print() {
    let mut tree = RBTree::new();

    for i in (1..100).rev() {
        tree.insert(i,i);
    }
    debug_breadth_print(&tree);
}
