/// A Left-leaning Red-Black Tree.
///
/// Sedgewick's algorithm from: https://www.cs.princeton.edu/~rs/talks/LLRB/RedBlack.pdf

use std::fmt::Debug;
use std::mem;
use std::cmp::Ordering::*;
use std::io::{stdout, Write};

use self::Color::*;
#[derive(Debug, PartialEq, Clone)]
enum Color {
    Black,
    Red,
}

/// Left Leaning Red-Black Tree
pub struct RBTree<K,V> where K: Ord {
    root: Option<Box<Node<K,V>>>,
}

impl<K,V> RBTree<K,V> where K: Ord {
    pub fn new() -> Self {
        RBTree {
            root: None,
        }
    }

    pub fn get(&self, key: &K) -> Option<&V> {
        if let Some(n) = self.root.as_ref() {
            return n.get(key)
        }
        None
    }

    pub fn min(&self) -> Option<&V> {
        if let Some(n) = self.root.as_ref() {
            return n.min()
        }
        None
    }

    pub fn max(&self) -> Option<&V> {
        if let Some(n) = self.root.as_ref() {
            return n.max()
        }
        None
    }

    pub fn insert(&mut self, key: K, value: V) -> Option<V> {
        if let Some(n) = self.root.as_mut() {
            return n.insert(key, value)
        }
        self.root = Some(Node::new_boxed(key, value, Red));
        None
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
        print!("{:?} ", n.key);

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


/// Node ------------------
#[derive(Debug)]
struct Node<K,V> where K: Ord {
    key: K,
    value: V,
    color: Color,
    left: Option<Box<Node<K,V>>>,
    right: Option<Box<Node<K,V>>>,
}

impl<K,V> Node<K,V> where K: Ord {
    fn new_boxed(k: K, v: V, color: Color) -> Box<Node<K,V>> {
        Box::new(Self::new(k,v,color))
    }

    fn new(k: K, v: V, color: Color) -> Self {
        Node {
            key: k,
            value: v,
            left: None,
            right: None,
            color: color,
        }
    }
}

/// BoxedNode ------------------
trait BoxedNode {
    type K;
    type V;
    fn get(&self, key: &Self::K) -> Option<&Self::V>;
    fn min(&self) -> Option<&Self::V>;
    fn max(&self) -> Option<&Self::V>;
    fn insert(&mut self, key: Self::K, value: Self::V) -> Option<Self::V>;
    fn is_red(&self) -> bool;
    fn is_black(&self) -> bool {
        !self.is_red()
    }
    // helpers
    fn flip_red(&mut self);
    fn rotate_left(&mut self);
    fn rotate_right(&mut self);
}

/// helper for red option
fn is_red<K,V>(n: &Option<Box<Node<K,V>>>) -> bool where K:Ord {
    match n.as_ref() {
        Some(n) => n.is_red(),
        None => false,
    }
}

/// helper for red option followed by a left red
fn is_red_left_red<K,V>(n: &Option<Box<Node<K,V>>>) -> bool where K:Ord {
    if let Some(n) = n.as_ref() {
        if n.is_red() {
            if let Some(n) = n.left.as_ref() {
                return n.is_red()
            }
        }
    }
    false
}

///
impl<K,V> BoxedNode for Box<Node<K,V>> where K: Ord {
    type K = K;
    type V = V;

    fn is_red(&self) -> bool {
        self.color == Red
    }

    fn get(&self, key: &K) -> Option<&V> {
        let mut curr = self;
        loop {
            let next;
            match key.cmp(&curr.key) {
                Equal => { return Some(&curr.value) }
                Less => { next = &curr.left; }
                Greater => { next = &curr.right; }
            }
            match *next {
                Some(ref node) => { curr = node }
                None => { return None }
            }
        }
    }

    fn min(&self) -> Option<&V> {
        let mut curr = self;
        loop {
            let next = &curr.left;
            match *next {
                Some(ref node) => { curr = node }
                None => { return Some(&curr.value) }
            }
        }
    }

    fn max(&self) -> Option<&V> {
        let mut curr = self;
        loop {
            let next = &curr.right;
            match *next {
                Some(ref node) => { curr = node }
                None => { return Some(&curr.value) }
            }
        }
    }

    fn insert(&mut self, key: K, value: V) -> Option<V> {
        // break 4-nodes
        if let (Some(_), Some(_)) = (self.left.as_ref(), self.right.as_ref()) {
            if self.left.as_ref().unwrap().is_red() && self.right.as_ref().unwrap().is_red() {
                self.flip_red();
            }
        }

        let ret;
        {
            let next;
            match key.cmp(&self.key) {
                Equal => { // replace value
                    let mut old = value;
                    mem::swap(&mut self.value, &mut old);
                    return Some(old);
                }
                Less => { next = &mut self.left; }
                Greater => { next = &mut self.right; }
            }
            match *next { // recurse or insert here
                Some(ref mut node) => {
                    ret = node.insert(key, value);
                }
                None => {
                    *next = Some(Node::new_boxed(key, value, Red));
                    return None;
                }
            }
        }

        // fix right-leaning red
        if is_red(&self.right) {
            self.rotate_left()
        }
        // fix two reds in a row
        if is_red_left_red(&self.left) {
            self.rotate_right()
        }
        ret
    }

    /// split "4-nodes"
    fn flip_red(&mut self) {
        self.color = Red;
        self.left.as_mut().unwrap().color = Black;
        self.right.as_mut().unwrap().color = Black;
    }

    /// rotate left to fix right-leaning red
    fn rotate_left(&mut self) {
        let mut right = self.right.take();
        let mut right_left = right.as_mut().unwrap().left.take();
        // perform rotation
        mem::swap(&mut self.right, &mut right_left);
        mem::swap(self, &mut right.as_mut().unwrap());
        mem::swap(&mut right, &mut self.left);
        self.color = self.left.as_ref().unwrap().color.clone();
        self.left.as_mut().unwrap().color = Red;
    }

    /// rotate right to fix red-red egde
    fn rotate_right(&mut self) {
        let mut left = self.left.take();
        let mut left_right = left.as_mut().unwrap().right.take();
        // perform rotation
        mem::swap(&mut self.left, &mut left_right);
        mem::swap(self, &mut left.as_mut().unwrap());
        mem::swap(&mut left, &mut self.right);
        self.color = self.right.as_ref().unwrap().color.clone();
        self.right.as_mut().unwrap().color = Red;
    }
}

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
fn test_debug_breadth_print() {
    let mut tree = RBTree::new();

    for i in (1..100).rev() {
        tree.insert(i,i);
    }
    debug_breadth_print(&tree);
}
