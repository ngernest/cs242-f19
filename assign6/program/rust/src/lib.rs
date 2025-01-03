#![allow(dead_code, unused_imports, unused_variables)]

use std::fmt::{Debug, Display};
use std::{fmt, mem};

#[derive(PartialEq, Eq, Clone)]
pub enum BinaryTree<T> {
    Leaf,
    Node(T, Box<BinaryTree<T>>, Box<BinaryTree<T>>),
}

impl<T: Debug + Display + PartialOrd> BinaryTree<T> {
    pub fn len(&self) -> usize {
        match self {
            BinaryTree::Leaf => 0,
            BinaryTree::Node(_x, l, r) => 1 + l.len() + r.len(),
        }
    }

    pub fn to_vec(&self) -> Vec<&T> {
        match self {
            BinaryTree::Leaf => vec![],
            BinaryTree::Node(x, l, r) => [l.to_vec(), vec![x], r.to_vec()].concat(),
        }
    }

    pub fn sorted(&self) -> bool {
        match self {
            BinaryTree::Leaf => true,
            BinaryTree::Node(x, l, r) => {
                let ls = l.to_vec();
                let rs = r.to_vec();
                l.sorted()
                    && r.sorted()
                    && ls.iter().all(|&y| *y < *x)
                    && rs.iter().all(|&z| *z > *x)
            }
        }
    }

    pub fn insert(&mut self, t: T) {
        match self {
            BinaryTree::Leaf => {
                *self = BinaryTree::Node(t, Box::new(BinaryTree::Leaf), Box::new(BinaryTree::Leaf))
            }
            BinaryTree::Node(x, l, r) => {
                if t < *x {
                    l.insert(t)
                } else if t > *x {
                    r.insert(t)
                }
            }
        }
    }

    pub fn search(&self, query: &T) -> Option<&T> {
        match self {
            BinaryTree::Leaf => None,
            BinaryTree::Node(x, l, r) => match l.search(query) {
                Some(y) => Some(y),
                None => {
                    if *x >= *query {
                        Some(x)
                    } else {
                        r.search(query)
                    }
                }
            },
        }
    }

    // Note: we only need to implement one single rebalancing step for this HW
    fn rotate_right(&mut self) {
        todo!()
    }

    fn rotate_left(&mut self) {
        todo!()
    }

    pub fn rebalance(&mut self) {
        match self {
            BinaryTree::Leaf => (),
            BinaryTree::Node(x, l, r) => {
                let balance_factor = (l.len() - r.len()) as i32;
                if balance_factor <= -2 {
                    // `r` taller than `l`, rebalance from right to left
                } else if balance_factor >= 2 {
                    // `l` taller than `r`, rebalance from left to right
                } else {
                    // No rebalancing needed, balance factor between -1 & 1
                    ()
                }
            }
        }
    }

    // Adapted from https://github.com/bpressure/ascii_tree
    fn fmt_levels(&self, f: &mut fmt::Formatter<'_>, level: Vec<usize>) -> fmt::Result {
        use BinaryTree::*;
        const EMPTY: &str = "   ";
        const EDGE: &str = " └─";
        const PIPE: &str = " │ ";
        const BRANCH: &str = " ├─";

        let maxpos = level.len();
        let mut second_line = String::new();
        for (pos, l) in level.iter().enumerate() {
            let last_row = pos == maxpos - 1;
            if *l == 1 {
                if !last_row {
                    write!(f, "{}", EMPTY)?
                } else {
                    write!(f, "{}", EDGE)?
                }
                second_line.push_str(EMPTY);
            } else {
                if !last_row {
                    write!(f, "{}", PIPE)?
                } else {
                    write!(f, "{}", BRANCH)?
                }
                second_line.push_str(PIPE);
            }
        }

        match self {
            Node(s, l, r) => {
                let mut d = 2;
                write!(f, " {}\n", s)?;
                for t in &[l, r] {
                    let mut lnext = level.clone();
                    lnext.push(d);
                    d -= 1;
                    t.fmt_levels(f, lnext)?;
                }
            }
            Leaf => write!(f, "\n")?,
        }
        Ok(())
    }
}

impl<T: Debug + Display + PartialOrd> fmt::Debug for BinaryTree<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.fmt_levels(f, vec![])
    }
}

#[cfg(test)]
mod test {
    use super::BinaryTree::*;
    use crate::BinaryTree;
    use lazy_static::lazy_static;

    lazy_static! {
        static ref TEST_TREE: BinaryTree<&'static str> = {
            Node(
                "B",
                Box::new(Node("A", Box::new(Leaf), Box::new(Leaf))),
                Box::new(Node("C", Box::new(Leaf), Box::new(Leaf))),
            )
        };
    }

    #[test]
    fn len_test() {
        assert_eq!(TEST_TREE.len(), 3);
    }

    #[test]
    fn to_vec_test() {
        assert_eq!(TEST_TREE.to_vec(), vec![&"A", &"B", &"C"]);
    }

    #[test]
    fn sorted_test() {
        let mut t = TEST_TREE.clone();
        assert!(t.sorted());

        t = Node("D", Box::new(Leaf), Box::new(t));
        assert!(!t.sorted());
    }

    #[test]
    fn insertion_test() {
        let mut t = TEST_TREE.clone();
        t.insert("E");
        assert!(t.sorted());
    }

    #[test]
    fn search_test() {
        let mut t = TEST_TREE.clone();
        t.insert("E");
        assert!(t.search(&"D") == Some(&"E"));
        assert!(t.search(&"C") == Some(&"C"));
        assert!(t.search(&"F") == None);
    }

    #[test]
    fn rebalance1_test() {
        let mut t = Node(
            "D",
            Box::new(Node(
                "B",
                Box::new(Node("A", Box::new(Leaf), Box::new(Leaf))),
                Box::new(Node("C", Box::new(Leaf), Box::new(Leaf))),
            )),
            Box::new(Node("E", Box::new(Leaf), Box::new(Leaf))),
        );

        let t2 = Node(
            "C",
            Box::new(Node(
                "B",
                Box::new(Node("A", Box::new(Leaf), Box::new(Leaf))),
                Box::new(Leaf),
            )),
            Box::new(Node(
                "D",
                Box::new(Leaf),
                Box::new(Node("E", Box::new(Leaf), Box::new(Leaf))),
            )),
        );

        t.rebalance();
        assert_eq!(t, t2);
    }

    #[test]
    fn rebalance2_test() {
        let mut t = Node(
            "A",
            Box::new(Leaf),
            Box::new(Node(
                "B",
                Box::new(Leaf),
                Box::new(Node(
                    "C",
                    Box::new(Leaf),
                    Box::new(Node("D", Box::new(Leaf), Box::new(Leaf))),
                )),
            )),
        );

        let t2 = Node(
            "B",
            Box::new(Node("A", Box::new(Leaf), Box::new(Leaf))),
            Box::new(Node(
                "C",
                Box::new(Leaf),
                Box::new(Node("D", Box::new(Leaf), Box::new(Leaf))),
            )),
        );

        t.rebalance();
        assert_eq!(t, t2);
    }

    #[test]
    fn rebalance3_test() {
        let mut t = Node(
            "E",
            Box::new(Node(
                "B",
                Box::new(Leaf),
                Box::new(Node(
                    "D",
                    Box::new(Node("C", Box::new(Leaf), Box::new(Leaf))),
                    Box::new(Leaf),
                )),
            )),
            Box::new(Node("F", Box::new(Leaf), Box::new(Leaf))),
        );

        let t2 = Node(
            "D",
            Box::new(Node(
                "B",
                Box::new(Leaf),
                Box::new(Node("C", Box::new(Leaf), Box::new(Leaf))),
            )),
            Box::new(Node(
                "E",
                Box::new(Leaf),
                Box::new(Node("F", Box::new(Leaf), Box::new(Leaf))),
            )),
        );

        t.rebalance();
        assert_eq!(t, t2);
    }
}
