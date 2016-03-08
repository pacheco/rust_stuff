/// B-tree implementation with single-pass insertion and
/// deletion. Algorithm from
/// [http://staff.ustc.edu.cn/~csli/graduate/algorithms/book6/chap19.htm].
///
/// A B-tree has an order `m`, the maximum number of children an inner node can hold.
///
/// Invariants:
///
/// - Every node (except the root) should have at least `m/2 - 1` keys and at most `m-1` keys
///
/// - For _single-pass insertion_, full nodes need to be split before being recursed in. Root is a special case (create new root and split).
///
/// - For _single-pass deletion_, a node needs to have at least `m/2` keys before being recursed in.

#[cfg(test)]
mod test;

use std::mem;
use std::vec;

/// BTree root. `t` is the minimum degree.
pub struct BTree<K, V> where K: Ord {
    height: usize,
    m: usize,
    count: usize,
    root: Box<Node<K, V>>,
}

/// BTree node
struct Node<K, V>  where K: Ord {
    keys: Vec<K>,
    // boxed nodes add a level of indirection but use much less memory if the vector is not full
    children: Vec<Box<Node<K, V>>>,
    values: Vec<V>,
}


impl<K, V> BTree<K, V> where K: Ord {
    /// Empty BTree with an order of 10
    pub fn new() -> Self {
        Self::new_with_order(10)
    }

    /// Empty BTree of the given order. Minimum order is 4 (a 2-3 tree).
    ///
    pub fn new_with_order(m: usize) -> Self {
        // min order is 4 (2-3 tree)
        let m = if m < 4 { 4 } else { m };
        BTree {
            height: 1,
            m: m,
            count: 0,
            root: Node::new_boxed(m),
        }
    }

    /// Return Some(value) corresponding to the key or None
    pub fn get(&self, key: &K) -> Option<&V> {
        return self.root.get(key);
    }

    pub fn height(&self) -> usize {
        self.height
    }

    /// Inserts an element, returning the older value or None
    pub fn insert(&mut self, key: K, value: V) -> Option<V> {
        // if root is full, split it first
        if self.root.is_full(self.m) {
            self.height += 1;
            let mut r = Node::new_boxed(self.m);
            mem::swap(&mut r, &mut self.root);
            self.root.children.push(r);
            self.root.split_child(self.m, 0);
        }
        let v = self.root.insert(self.m, key, value);
        match v {
            None => self.count += 1,
            _ => {}
        }
        v
    }

    pub fn len(&self) -> usize {
        self.count
    }

    pub fn remove(&mut self, key: &K) -> Option<V> {
        let kv = self.root.remove(self.m, key);
        if self.root.keys.is_empty() && !self.root.children.is_empty() {
            debug_assert_eq!(self.root.children.len(), 1);
            self.root = self.root.children.pop().unwrap();
            self.height -= 1;
        }

        match kv {
            Some((_,v)) => Some(v),
            None => None,
        }
    }
}

impl<K, V> Node<K, V> where K: Ord {
    /// Create a new node already Boxed
    fn new_boxed(m: usize) -> Box<Self> {
        Box::new(Node {
            keys: Vec::with_capacity(m - 1),
            values: Vec::with_capacity(m - 1),
            children: Vec::with_capacity(m),
        })
    }

    fn is_leaf(&self) -> bool {
        return self.children.is_empty();
    }

    fn is_full(&self, m: usize) -> bool {
        self.keys.len() == m-1
    }

    fn is_too_small(&self, m: usize) -> bool {
        // joining two nodes should be < full
        self.keys.len() < (m/2)
    }

    fn get(&self, key: &K) -> Option<&V> {
        let mut curr = self;
        loop {
            match curr.keys.binary_search(key) {
                Ok(n) => {
                    return Some(&curr.values[n]);
                }
                Err(n) => {
                    if curr.is_leaf() {
                        return None;
                    }
                    curr = &*curr.children[n];
                }
            }
        }
    }

    /// Internal insert used by the BTree.insert() method
    // TODO: non-recursive version? tree height is log(len), seems not necessary
    fn insert(&mut self, m: usize, key: K, value: V) -> Option<V> {
        debug_assert!(!self.is_full(m));
        let mut value = value;
        let mut curr = self;

        match curr.keys.binary_search(&key) {
            Ok(n) => {
                mem::swap(&mut curr.values[n], &mut value);
                return Some(value);
            }
            Err(n) => {
                if curr.is_leaf() {
                    // leaf, insert item
                    curr.keys.insert(n, key);
                    curr.values.insert(n, value);
                    return None;
                } else {
                    // inner node
                    if curr.children[n].is_full(m) {
                        // child we need to recurse on is full, split it
                        curr.split_child(m, n);
                        if key < curr.keys[n] {
                            curr.children[n].insert(m, key, value)
                        } else {
                            curr.children[n+1].insert(m, key, value)
                        }
                    } else {
                        curr.children[n].insert(m, key, value)
                    }
                }
            }
        }
    }

    /// Used to do single pass insertions. Full nodes are split while
    /// going down the tree. This method expects the given child to be
    /// full and the node (parent) to be _not_ full
    fn split_child(&mut self, m: usize, child_idx: usize) {
        let mkey: K;
        let mval: V;
        let mut sibling = Node::new_boxed(m);
        // new block just so we can borrow into `child` to make the code nicer
        {
            let child = &mut self.children[child_idx];
            debug_assert!(child.is_full(m));

            // move keys/values after median to sibling
            // TODO: reallocating new arrays... use unsafe and copy instead? mem::move?
            let median = (m+1)/2;
            sibling.keys = child.keys.split_off(median);
            sibling.values = child.values.split_off(median);

            // median kv
            mkey = child.keys.pop().unwrap();
            mval = child.values.pop().unwrap();
            // move children after median to sibling
            if !child.is_leaf() {
                sibling.children = child.children.split_off(median);
            }
        }

        // insert median and new sibling in parent
        self.keys.insert(child_idx, mkey);
        self.values.insert(child_idx, mval);
        self.children.insert(child_idx + 1, sibling);
    }

    pub fn remove(&mut self, m: usize, key: &K) -> Option<(K,V)> {
        match self.keys.binary_search(key) {
            Ok(n) => { // found item in node
                if self.is_leaf() {
                    Some((self.keys.remove(n), self.values.remove(n)))
                } else {
                    // here we're removing the key from an inner
                    // node. We need to "raise" a key from either left
                    // or right side, if any of them is larger then
                    // the minimum size. If both are minimal, merge
                    // them plus the removed key and recursively
                    // delete on the merged node. We use `unsafe` to
                    // get an immutable reference to the key we will
                    // move up - we need to call delete() recursivelly
                    if !self.children[n].is_too_small(m) {
                        // take item from left
                        let pred_key: &K;
                        unsafe {
                            pred_key = &*(self.children[n].keys.last().unwrap() as *const K);
                        }
                        let (mut k, mut v) = self.children[n].remove(m, pred_key).unwrap();
                        mem::swap(&mut self.keys[n], &mut k);
                        mem::swap(&mut self.values[n], &mut v);
                        Some((k,v))
                    } else if !self.children[n+1].is_too_small(m) {
                        // take item from right
                        let succ_key: &K;
                        unsafe {
                            succ_key = &*(self.children[n+1].keys.first().unwrap() as *const K);
                        }
                        let (mut k, mut v) = self.children[n+1].remove(m, succ_key).unwrap();
                        mem::swap(&mut self.keys[n], &mut k);
                        mem::swap(&mut self.values[n], &mut v);
                        Some((k,v))
                    } else { // merge nodes
                        let k = self.keys.remove(n);
                        let v = self.values.remove(n);
                        let mut deleted_node = self.children.remove(n+1);
                        self.children[n].keys.push(k);
                        self.children[n].values.push(v);
                        self.children[n].keys.append(&mut deleted_node.keys);
                        self.children[n].values.append(&mut deleted_node.values);
                        self.children[n].children.append(&mut deleted_node.children);
                        self.children[n].remove(m, key)
                    }
                }
            }
            Err(n) => { // did not find item in node
                if self.is_leaf() {
                    None
                } else {
                    // make sure node is large enough before recursing
                    if self.children[n].is_too_small(m) {
                        if n > 0 && !self.children[n-1].is_too_small(m) { // take from left
                            // move a key down to node
                            let k = self.keys.remove(n-1);
                            let v = self.values.remove(n-1);
                            self.children[n].keys.insert(0, k);
                            self.children[n].values.insert(0, v);
                            // move a key up from left sibling
                            let k = self.children[n-1].keys.pop().unwrap();
                            let v = self.children[n-1].values.pop().unwrap();
                            self.keys.insert(n-1, k);
                            self.values.insert(n-1, v);
                            // move child from left sibling
                            if !self.children[n-1].is_leaf() {
                                let c = self.children[n-1].children.pop().unwrap();
                                self.children[n].children.insert(0,c);
                            }
                        }
                        else if n < self.keys.len() && !self.children[n+1].is_too_small(m) { // take from right
                            // move a key down to node
                            let k = self.keys.remove(n);
                            let v = self.values.remove(n);
                            self.children[n].keys.push(k);
                            self.children[n].values.push(v);
                            // move a key up from right sibling
                            let k = self.children[n+1].keys.remove(0);
                            let v = self.children[n+1].values.remove(0);
                            self.keys.insert(n, k);
                            self.values.insert(n, v);
                            // move child from right sibling
                            if !self.children[n+1].is_leaf() {
                                let c = self.children[n+1].children.remove(0);
                                self.children[n].children.push(c);
                            }
                        } else {
                            if n > 0 { // merge with left sibling
                                // move a key down as new median
                                let k = self.keys.remove(n-1);
                                let v = self.values.remove(n-1);
                                self.children[n-1].keys.push(k);
                                self.children[n-1].values.push(v);
                                // merge node
                                let mut removed_node = self.children.remove(n);
                                self.children[n-1].keys.append(&mut removed_node.keys);
                                self.children[n-1].values.append(&mut removed_node.values);
                                self.children[n-1].children.append(&mut removed_node.children);
                                // corner case where `n` changes
                                return self.children[n-1].remove(m,key);
                            } else { // merge with right sibling
                                // move a key down as new median
                                let k = self.keys.remove(n);
                                let v = self.values.remove(n);
                                self.children[n].keys.push(k);
                                self.children[n].values.push(v);
                                // merge node
                                let mut removed_node = self.children.remove(n+1);
                                self.children[n].keys.append(&mut removed_node.keys);
                                self.children[n].values.append(&mut removed_node.values);
                                self.children[n].children.append(&mut removed_node.children);
                            }
                        }
                    }
                    self.children[n].remove(m, key)
                }
            }
        }
    }
}


// Iterators ---------------------------------------------

struct NodeIter<'a, K, V> where K: 'a + Ord, V: 'a {
    node: &'a Node<K, V>,
    next_val: usize,
    go_child: bool,
}

pub struct Iter<'a, K, V> where K: 'a + Ord, V: 'a {
    stack: Vec<NodeIter<'a, K, V>>,
    curr: NodeIter<'a, K, V>,
}

impl<'a, K, V> Iterator for Iter<'a, K, V> where K: 'a + Ord, V: 'a {
    type Item = (&'a K, &'a V);

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if self.curr.node.is_leaf() {
                // leaf, consume a value or pop node from stack
                if self.curr.next_val < self.curr.node.values.len() {
                    let i = self.curr.next_val;
                    self.curr.next_val += 1;
                    return Some((&self.curr.node.keys[i], &self.curr.node.values[i])); // consume a value
                } else {
                    // pop from stack
                    match self.stack.pop() {
                        Some(x) => {
                            self.curr = x;
                            continue;
                        }
                        None => {
                            return None;
                        }
                    }
                }
            } else {
                // non-leaf, either go to child, consume a value or pop node from stack
                if self.curr.go_child {
                    // go to child
                    self.curr.go_child = false;
                    let mut tmp = NodeIter {
                        node: &self.curr.node.children[self.curr.next_val],
                        next_val: 0,
                        go_child: true,
                    };
                    mem::swap(&mut tmp, &mut self.curr);
                    self.stack.push(tmp);
                    continue;
                } else {
                    // try to consume a value
                    if self.curr.next_val < self.curr.node.values.len() {
                        self.curr.go_child = true;
                        let i = self.curr.next_val;
                        self.curr.next_val += 1;
                        return Some((&self.curr.node.keys[i], &self.curr.node.values[i])); // consume a value
                    } else {
                        // pop from stack
                        match self.stack.pop() {
                            Some(x) => {
                                self.curr = x;
                                continue;
                            }
                            None => {
                                return None;
                            }
                        }
                    }
                }
            }
        }
    }
}

impl<'a, K, V> BTree<K, V> where K: Ord {
    pub fn iter(&'a self) -> Iter<'a, K, V> {
        Iter {
            stack: vec![],
            curr: NodeIter {
                node: &self.root,
                next_val: 0,
                go_child: true,
            }
        }
    }
}

// TODO: implement a lazy version of the into iterator
impl<K, V> Node<K, V> where K: Ord {
    fn depth_first_collect_into<'a>(self, items: &mut Vec<(K,V)>) {
        let inner = !self.is_leaf();
        // TODO: using iterators because we can't move out of an indexed vec
        let mut children = self.children.into_iter();
        let keys = self.keys.into_iter();
        let values = self.values.into_iter();
        for kv in keys.zip(values) {
            if inner {
                children.next().unwrap().depth_first_collect_into(items);
            }
            items.push(kv);
        }
        if inner {
            children.next().unwrap().depth_first_collect_into(items);
        }
    }
}

impl<K, V> IntoIterator for BTree<K, V> where K: Ord {
    type Item = (K,V);
    type IntoIter = vec::IntoIter<(K,V)>;

    fn into_iter(self) -> Self::IntoIter {
        let mut items = vec![];
        self.root.depth_first_collect_into(&mut items);
        items.into_iter()
    }
}
