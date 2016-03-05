use std::mem;

pub struct RBTree<K,V> where K: Ord {
    root: Node<K,V>,
}

#[derive(Debug)]
struct Node<K,V> where K: Ord {
    key: K,
    value: V,
    left: Option<Box<Node<K,V>>>,
    right: Option<Box<Node<K,V>>>,
    black: bool,
}

impl<K,V> Node<K,V> where K: Ord {
    fn new_boxed(k: K, v: V, black: bool) -> Box<Self> {
        Box::new(Node {
            key: k,
            value: v,
            left: None,
            right: None,
            black: black,
        })
    }

    #[inline]
    fn is_black(&self) -> bool {
        self.black
    }
    #[inline]
    fn is_red(&self) -> bool {
        !self.black
    }

    fn push_black(&mut self) {
        assert!(self.is_black());
        let left = self.left.as_mut().unwrap();
        let right = self.right.as_mut().unwrap();
        assert!(left.is_red());
        assert!(right.is_red());
        self.black = false;
        left.black = true;
        right.black = true;
    }

    fn pull_black(&mut self) {
        assert!(self.is_red());
        let left = self.left.as_mut().unwrap();
        let right = self.right.as_mut().unwrap();
        assert!(left.is_black());
        assert!(right.is_black());
        self.black = true;
        left.black = false;
        right.black = false;
    }

    fn flip_left(&mut self) {
        // swap colors with right
        mem::swap(&mut self.black, &mut self.right.as_mut().unwrap().black);
        let mut right = self.right.take();
        let mut right_left = right.as_mut().unwrap().left.take();
        // perform rotation
        mem::swap(&mut self.right, &mut right_left);
        mem::swap(self, &mut right.as_mut().unwrap());
        mem::swap(&mut right, &mut self.left);
    }

    fn flip_right(&mut self) {
        // swap colors with right
        mem::swap(&mut self.black, &mut self.left.as_mut().unwrap().black);
        let mut left = self.left.take();
        let mut left_right = left.as_mut().unwrap().right.take();
        // perform rotation
        mem::swap(&mut self.left, &mut left_right);
        mem::swap(self, &mut left.as_mut().unwrap());
        mem::swap(&mut left, &mut self.right);
    }
}

#[test]
fn test_flip_left() {
    let mut n = Node::new_boxed(0,0,true);
    let mut l = Node::new_boxed(1,1,true);
    let mut r = Node::new_boxed(2,2,false);
    let mut rl = Node::new_boxed(3,3,true);
    let mut rr = Node::new_boxed(4,4,true);

    r.as_mut().left = Some(rl);
    r.as_mut().right = Some(rr);
    n.as_mut().left = Some(l);
    n.as_mut().right = Some(r);

    println!("{:?}", n);
    n.as_mut().flip_left();
    println!("{:?}", n);
}

#[test]
fn test_flip_right() {
    let mut n = Node::new_boxed(0,0,true);
    let mut l = Node::new_boxed(1,1,false);
    let mut ll = Node::new_boxed(2,2,true);
    let mut lr = Node::new_boxed(3,3,true);
    let mut r = Node::new_boxed(4,4,true);

    l.as_mut().left = Some(ll);
    l.as_mut().right = Some(lr);
    n.as_mut().left = Some(l);
    n.as_mut().right = Some(r);

    println!("{:?}", n);
    n.as_mut().flip_right();
    println!("{:?}", n);
}
