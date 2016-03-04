/// B-tree implementation with single-pass insertion and deletion
///
/// A B-tree has an order `m`, the maximum number of children an inner node can hold.
///
/// Invariants:
///
/// - Every node (except the root) should have at least `floor(m/2)` keys and at most `m-1` keys
///
/// - A node with x keys has x+1 children
///
/// - For _single-pass insertion_, full nodes need to be split before being recursed in. Root is a special case (create new root and split).
///
/// - For _single-pass deletion_, a node needs to have at least `floor(m/2) + 1` keys before being recursed in (except root).

use std::mem;
use std::fmt::Debug;
use std::io::Write;
use std::io::stdout;

/// BTree root. `t` is the minimum degree.
#[derive(Debug)]
pub struct BTree<K: Ord + Debug, V: Debug> {
    m: usize,
    root: Box<Node<K, V>>,
}

/// BTree node
#[derive(Debug)]
struct Node<K: Ord + Debug, V: Debug> {
    keys: Vec<Box<K>>,
    values: Vec<Box<V>>,
    children: Vec<Box<Node<K, V>>>,
}


impl<K: Ord + Debug, V: Debug> BTree<K, V> {
    /// Empty BTree of the given order. Order must be at least 3 (2 also "works" but produces bad trees)
    pub fn new(order: usize) -> Self {
        assert!(order > 3);
        BTree {
            m: order,
            root: Node::new_boxed(order),
        }
    }

    /// Inserts an element, returning the older value or None
    pub fn insert(&mut self, key: K, value: V) -> Option<V> {
        // if root is full, split it first
        if self.root.keys.len() == self.m-1 {
            let mut r = Node::new_boxed(self.m);
            mem::swap(&mut r, &mut self.root);
            self.root.children.push(r);
            self.root.split_child(self.m, 0);
        }
        self.root.insert(self.m, key, value)
    }
}


impl<K: Ord + Debug, V: Debug> Node<K, V> {
    /// Create a new node already Boxed
    fn new_boxed(order: usize) -> Box<Self> {
        Box::new(Node {
            keys: Vec::with_capacity(order - 1),
            values: Vec::with_capacity(order - 1),
            children: Vec::with_capacity(order),
        })
    }

    /// Internal insert used by the BTree.insert() method
    fn insert(&mut self, order: usize, key: K, value: V) -> Option<V> {
        assert!(self.keys.len() < order-1);
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
                    let mut n = n;
                    if self.children[n].keys.len() == order-1 {
                        // child we need to recurse on is full, split it
                        self.split_child(order, n);
                        n += 1;
                    }
                    self.children[n].insert(order, *key, *value)
                }
            }
        }
    }

    /// Used to do single pass insertions. Full nodes are split while
    /// going down the tree. This method expects the given child to be
    /// full (order-1 elements).
    fn split_child(&mut self, order: usize, child_idx: usize) {
        let mkey: Box<K>;
        let mval: Box<V>;
        let mut sibling: Box<Self> = Node::new_boxed(order);
        // new block just so we can borrow into `child` to make the code nicer
        {
            let child = &mut self.children[child_idx];
            assert_eq!(child.keys.len(), order-1);

            // move keys/values after median to sibling
            // TODO: reallocating new arrays... use unsafe and copy instead? mem::move?
            let median = (order+1)/2;
            if order > 2 { // corner case of having a single key
                sibling.keys = child.keys.split_off(median);
                sibling.values = child.values.split_off(median);
            }
            // median kv
            mkey = child.keys.pop().unwrap();
            mval = child.values.pop().unwrap();
            // move children after median to sibling
            if !child.children.is_empty() {
                sibling.children = child.children.split_off(median);
            }
        }

        // insert median and new sibling in parent
        self.keys.insert(child_idx, mkey);
        self.values.insert(child_idx, mval);
        self.children.insert(child_idx + 1, sibling);
    }
}


// ----- Debug printing functions -----------------------------------------------------
impl<K: Ord + Debug, V: Debug> BTree<K, V>{
    /// Print keys in breath first order. Same level keys are printed on the same line
    pub fn breath_first_print(&self) {
        let mut nodes: Vec<&Box<Node<K,V>>> = vec![];
        let mut height = 0;
        let mut height_nodes = 1; // tracks how many nodes we still need to pop in this height
        let mut next_height_nodes = 0; // accumulator for the number of nodes on the next height
        nodes.insert(0, &self.root);
        while !nodes.is_empty() {
            let n = nodes.pop().unwrap();
            height_nodes -= 1;
            next_height_nodes += n.children.len();
            print!("{:?}", n.keys);
            if height_nodes == 0 {
                // finished printing this height
                height += 1;
                height_nodes = next_height_nodes;
                next_height_nodes = 0;
                println!("{}", height);
            }
            for c in &n.children {
                nodes.insert(0, c);
            }
        }
        stdout().flush().unwrap();
    }

    /// Print keys in order
    pub fn depth_first_print(&self) {
        self.root.depth_first_print();
    }
}

impl<K: Ord + Debug, V: Debug> Node<K, V> {
    fn depth_first_print(&self) {
        let mut n = 0;
        while n < self.keys.len() {
            if !self.children.is_empty() {
                let c = &self.children[n];
                c.depth_first_print();
            }
            println!("{:?}", self.keys[n]);
            n += 1;
        }
        if !self.children.is_empty() {
            self.children[n].depth_first_print();
        }
    }
}

// ------ Tests ----------------------------------------
#[test]
fn test() {
}
