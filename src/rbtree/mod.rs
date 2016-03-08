/// A Left-leaning Red-Black Tree.
///
/// Sedgewick's algorithm from: https://www.cs.princeton.edu/~rs/talks/LLRB/LLRB.pdf

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

impl Color {
    fn inverse(&self) -> Color {
        match *self {
            Black => Red,
            Red => Black,
        }
    }
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
        self.root.get(key)
    }

    pub fn min(&self) -> Option<&V> {
        self.root.min()
    }

    pub fn max(&self) -> Option<&V> {
        self.root.max()
    }

    pub fn insert(&mut self, key: K, value: V) -> Option<V> {
        self.root.insert(key,value)
    }
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
    fn new_boxed(k: K, v: V, color: Color) -> Option<Box<Node<K,V>>> {
        Some(Box::new(Self::new(k,v,color)))
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

    /// "split" a 4-node
    fn color_flip(&mut self) {
        self.color = self.color.inverse();
        if let Some(n) = self.left.as_mut() {
            n.color = n.color.inverse();
        }
        if let Some(n) = self.right.as_mut() {
            n.color = n.color.inverse();
        }
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

    fn move_red_right(&mut self) {
        self.color_flip();
        if self.left.as_ref().unwrap().left.is_red() {
            self.rotate_right();
            self.color_flip();
        }
    }

    fn move_red_left(&mut self) {
        self.color_flip();
        if self.right.as_ref().unwrap().left.is_red() {
            self.right.as_mut().unwrap().rotate_right();
            self.rotate_left();
            self.color_flip();
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
    fn remove_max(&mut self) -> Option<Self::V>;
    fn remove_min(&mut self) -> Option<Self::V>;
    // helpers
    fn is_red(&self) -> bool;
}

///
impl<K,V> BoxedNode for Option<Box<Node<K,V>>> where K: Ord {
    type K = K;
    type V = V;

    fn is_red(&self) -> bool {
        match self.as_ref() {
            Some(n) => n.color == Red,
            None => false,
        }
    }

    fn get(&self, key: &K) -> Option<&V> {
        let mut curr = self;
        loop {
            match curr.as_ref() {
                Some(n) => {
                    match key.cmp(&n.key) {
                        Equal => { return Some(&n.value) }
                        Less => { curr = &n.left; }
                        Greater => { curr = &n.right; }
                    }
                }
                None => return None,
            }
        }
    }

    fn min(&self) -> Option<&V> {
        let mut curr = self;
        loop {
            match curr.as_ref() {
                Some(n) => {
                    match n.left {
                        Some(_) => curr = &n.left,
                        None => return Some(&n.value),
                    }
                }
                None => return None,
            }
        }
    }

    fn max(&self) -> Option<&V> {
        let mut curr = self;
        loop {
            match curr.as_ref() {
                Some(n) => {
                    match n.right.as_ref() {
                        Some(_) => curr = &n.right,
                        None => return Some(&n.value),
                    }
                }
                None => return None,
            }
        }
    }

    fn insert(&mut self, key: K, value: V) -> Option<V> {
        // can't use match Some(n) here because we assign to self...
        if let None = self.as_mut() {
            *self = Node::new_boxed(key, value, Red);
            return None;
        } else {
            let mut n = self.as_mut().unwrap();

            // break 4-nodes
            if n.left.is_red() && n.right.is_red() {
                n.color_flip();
            }

            let ret;
            match key.cmp(&n.key) {
                Equal => { // replace value
                    let mut old = value;
                    mem::swap(&mut n.value, &mut old);
                    ret = Some(old);
                }
                Less => {
                    ret = n.left.insert(key,value);
                }
                Greater => {
                    ret = n.right.insert(key,value);
                }
            }

            // fix right-leaning red
            if n.right.is_red() {
                n.rotate_left();
            }
            // fix two reds in a row
            if n.left.is_red() && n.left.as_ref().unwrap().left.is_red() {
                n.rotate_right();
            }

            return ret;
        }
    }

    fn remove_max(&mut self) -> Option<V> {
        let mut remove_self = false;
        let mut ret = None;
        match self.as_mut() {
            None => return None,
            Some(n) => {
                // lean 3-nodes to the right
                if n.left.is_red() {
                    n.rotate_right();
                }
                if n.right.is_none() {
                    remove_self = true;
                } else {
                    if !n.right.is_red() && !n.right.as_ref().unwrap().left.is_red() {
                        n.move_red_right();
                    }
                    ret = n.right.remove_max();
                    // fix right-leaning red
                    if n.right.is_red() {
                        n.rotate_left();
                    }
                    // fix two reds in a row
                    if n.left.is_red() && n.left.as_ref().unwrap().left.is_red() {
                        n.rotate_right();
                    }
                    // break 4-nodes
                    if n.left.is_red() && n.right.is_red() {
                        n.color_flip();
                    }
                }
            }
        }

        if remove_self {
            let old = self.take();
            return Some(old.unwrap().value);
        } else {
            return ret;
        }
    }

    fn remove_min(&mut self) -> Option<V> {
        let mut remove_self = false;
        let mut ret = None;
        match self.as_mut() {
            None => return None,
            Some(n) => {
                if n.left.is_none() {
                    remove_self = true;
                } else {
                    if !n.left.is_red() && !n.left.as_ref().unwrap().left.is_red() {
                        n.move_red_left();
                    }
                    ret = n.left.remove_min();
                    // fix right-leaning red
                    if n.right.is_red() {
                        n.rotate_left();
                    }
                    // fix two reds in a row
                    if n.left.is_red() && n.left.as_ref().unwrap().left.is_red() {
                        n.rotate_right();
                    }
                    // break 4-nodes
                    if n.left.is_red() && n.right.is_red() {
                        n.color_flip();
                    }
                }
            }
        }

        if remove_self {
            let old = self.take();
            return Some(old.unwrap().value);
        } else {
            return ret;
        }
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
fn test_remove_max() {
    let mut tree = RBTree::new();

    for i in (1..1000).rev() {
        tree.insert(i,i);
    }
    for i in (1..1000).rev() {
        assert_eq!(tree.root.remove_max(), Some(i));
    }
}

#[test]
fn test_remove_min() {
    let mut tree = RBTree::new();

    for i in (1..1000).rev() {
        tree.insert(i,i);
    }
    for i in 1..1000 {
        assert_eq!(tree.root.remove_min(), Some(i));
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
