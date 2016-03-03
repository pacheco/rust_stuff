/// B-tree implementation with single-pass insertion and deletion
///
/// A B-tree has a minimum degree, `t`, which is the minimum number of children a node can have, the maximum is `2t`.
///
/// Invariants:
///
/// - Every node (except the root) should have at least `t-1` keys and at most `2t-1` keys
///
/// - A node with x keys has x+1 children
///
/// - For _single-pass insertion_, full nodes need to be split before being recursed in. Root is a special case (create new root and split).
///
/// - For _single-pass deletion_, a node needs to have at least `t` keys before being recursed in (except root).

use std::mem;

/// BTree root. `t` is the minimum degree.
#[derive(Debug)]
pub struct BTree<K: Ord, V> {
    t: usize,
    root: Box<Node<K, V>>,
}

#[derive(Debug)]
struct Node<K: Ord, V> {
    keys: Vec<Box<K>>,
    values: Vec<Box<V>>,
    children: Vec<Box<Node<K, V>>>,
}

impl<K: Ord, V> BTree<K, V> {
    /// Empty BTree of the given minimum degree
    pub fn new(min_degree: usize) -> Self {
        BTree {
            t: min_degree,
            root: Node::new_boxed(min_degree),
        }
    }

    /// Inserts an element, returning the older value or None
    pub fn insert(&mut self, key: K, value: V) -> Option<V> {
        // if root is full, split it first
        if self.root.keys.len() == 2*self.t-1 {
            let mut r = Node::new_boxed(self.t);
            mem::swap(&mut r, &mut self.root);
            self.root.children.push(r);
            self.root.split_child(self.t, 0);
        }
        self.root.insert(self.t, key, value)
    }
}

impl<K: Ord, V> Node<K, V> {
    fn new_boxed(t: usize) -> Box<Self> {
        Box::new(Node {
            keys: Vec::with_capacity(t*2 - 1),
            values: Vec::with_capacity(t*2 - 1),
            children: Vec::with_capacity(t*2),
        })
    }

    fn insert(&mut self, t: usize, key: K, value: V) -> Option<V> {
        assert!(self.children.len() < 2*t+1);
        let mut key = Box::new(key);
        let mut value = Box::new(value);
        if self.children.is_empty() {
            // leaf, insert item into current node
            match self.keys.binary_search(&key) {
                Ok(n) => {
                    mem::swap(&mut self.keys[n], &mut key);
                    mem::swap(&mut self.values[n], &mut value);
                    Some(*value)
                }
                Err(n) => {
                    self.keys.insert(n, key);
                    self.values.insert(n, value);
                    None
                }
            }
        } else {
            // inner node
            match self.keys.binary_search(&key) {
                Ok(n) => {
                    mem::swap(&mut self.keys[n], &mut key);
                    mem::swap(&mut self.values[n], &mut value);
                    Some(*value)
                }
                Err(n) => {
                    if self.children[n].children.len() == 2*t-1 {
                        // child we need to recurse on is full, split it
                        self.split_child(t, n);
                    }
                    self.children[n].insert(t, *key, *value)
                }
            }
        }
    }

    fn split_child(&mut self, t: usize, child_idx: usize) {
        let mkey: Box<K>;
        let mval: Box<V>;
        let mut sibling: Box<Self> = Node::new_boxed(t);
        // new block just so we can borrow into `child` to make the code nicer
        {
            let child = &mut self.children[child_idx];
            assert_eq!(child.keys.len(), 2*t-1);

            // move keys/values after median to sibling
            // TODO: reallocating new arrays... use unsafe and copy instead? mem::move?
            sibling.keys = child.keys.split_off(t);
            sibling.values = child.values.split_off(t);
            // median kv
            mkey = child.keys.pop().unwrap();
            mval = child.values.pop().unwrap();
            // move children after median to sibling
            sibling.children = child.children.split_off(t);
        }

        // insert median and new sibling in parent
        self.keys.insert(child_idx, mkey);
        self.values.insert(child_idx, mval);
        self.children.insert(child_idx + 1, sibling);
    }
}


// ----------------------------------------------
#[test]
fn test_split() {
    let mut r: BTree<i32,String> = BTree::new(1);
    println!("{:?}", r.insert(1, String::from("burzum")));
    println!("{:?}", r);
}
