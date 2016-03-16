/// A Left-leaning Red-Black Tree.
///
/// Sedgewick's algorithm from: https://www.cs.princeton.edu/~rs/talks/LLRB/LLRB.pdf

#[cfg(test)]
mod test;

use std::mem;
use std::cmp::Ordering::*;

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
    len: usize,
}

impl<K,V> RBTree<K,V> where K: Ord {
    pub fn new() -> Self {
        RBTree {
            root: None,
            len: 0,
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
        let ret = self.root.insert(key,value);
        if let None = ret {
            self.len += 1;
        }
        return ret;
    }

    pub fn remove(&mut self, key: &K) -> Option<V> {
        self.root.remove(key)
    }
}


/// Node ------------------
#[derive(Debug)]
struct Node<K,V> where K: Ord {
    key: K,
    value: Option<V>,
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
            value: Some(v),
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
    type K: Ord;
    type V;
    fn get(&self, key: &Self::K) -> Option<&Self::V>;
    fn min(&self) -> Option<&Self::V>;
    fn max(&self) -> Option<&Self::V>;
    fn insert(&mut self, key: Self::K, value: Self::V) -> Option<Self::V>;
    fn remove(&mut self, key: &Self::K) -> Option<Self::V>;
    // helpers
    fn remove_min(&mut self) -> Option<Box<Node<Self::K,Self::V>>>;
    fn is_red(&self) -> bool;
}

impl<K,V> Drop for Node<K,V> where K: Ord {
    fn drop(&mut self) {
        let mut to_drop = vec![];
        to_drop.push(mem::replace(&mut self.left, None));
        to_drop.push(mem::replace(&mut self.right, None));
        while let Some(next) = to_drop.pop() {
            if let Some(mut n) = next {
                to_drop.push(mem::replace(&mut n.left, None));
                to_drop.push(mem::replace(&mut n.right, None));
            }
        }
    }
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
                        Equal => { return n.value.as_ref() }
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
                        None => return n.value.as_ref(),
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
                        None => return n.value.as_ref(),
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
                    let mut old = Some(value);
                    mem::swap(&mut n.value, &mut old);
                    ret = old;
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

    fn remove_min(&mut self) -> Option<Box<Node<K,V>>> {
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
            return self.take();
        }
        return ret;
    }

    fn remove(&mut self, key: &K) -> Option<V> {
        let mut remove_self = false;
        let mut ret = None;
        match self.as_mut() {
            None => return None,
            Some(n) => {
                if *key < n.key {
                    if !n.left.is_red() && !n.left.as_ref().unwrap().left.is_red() {
                        n.move_red_left();
                    }
                    ret = n.left.remove(key);
                }
                else {
                    if n.left.is_red() {
                        n.rotate_right();
                    }
                    if *key == n.key && n.right.is_none() {
                        remove_self = true;
                    } else {
                        if !n.right.is_red() && !n.right.as_ref().unwrap().left.is_red() {
                            n.move_red_right();
                        }
                        if key == &n.key {
                            let mut min_right = n.right.remove_min();
                            mem::swap(n, &mut min_right.as_mut().unwrap());
                            ret = mem::replace(&mut min_right.unwrap().value, None);
                        }
                    }
                }
                if !remove_self {
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
        if remove_self { // modify self outside due to the borrow checker...
            let old = self.take();
            return mem::replace(&mut old.unwrap().value, None);
        }
        return ret;
    }
}
