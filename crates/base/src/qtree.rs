use std::{
    collections::{vec_deque, VecDeque},
    fmt::Display,
    hash::Hash,
    ops::{Shl, Shr},
};

use rustc_hash::FxHashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct QTreeKey(u64);

impl Hash for QTreeKey {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.0;
    }
}

impl QTreeKey {
    /// Returns the depth of the node. 0 <= depth <= 28.
    pub fn depth(&self) -> u8 {
        (self.0.shr(56) as u8) & 0x1fu8
    }

    /// Returns the x coordinate of the node. 0 <= x < 2^depth.
    pub fn x(&self) -> u32 {
        (self.0.shr(28) as u32) & 0xfff_ffffu32
    }

    /// Returns the y coordinate of the node. 0 <= y < 2^depth.
    pub fn y(&self) -> u32 {
        (self.0 as u32) & 0xfff_ffffu32
    }

    /// Returns the key of the parent node.
    pub fn parent(&self) -> Option<QTreeKey> {
        if self.depth() == 0 {
            None
        } else {
            QTreeKey::new(self.depth() - 1, self.x().shr(1), self.y().shr(1))
        }
    }

    pub fn left(&self) -> QTreeKey {
        QTreeKey::new(self.depth(), self.x().wrapping_sub(1), self.y()).unwrap()
    }

    pub fn right(&self) -> QTreeKey {
        QTreeKey::new(self.depth(), self.x().wrapping_add(1), self.y()).unwrap()
    }

    pub fn top(&self) -> QTreeKey {
        QTreeKey::new(self.depth(), self.x(), self.y().wrapping_sub(1)).unwrap()
    }

    pub fn bottom(&self) -> QTreeKey {
        QTreeKey::new(self.depth(), self.x(), self.y().wrapping_add(1)).unwrap()
    }

    /// Returns the key of the left top child node.
    pub fn child_lt(&self) -> Option<QTreeKey> {
        if self.depth() > 28 {
            None
        } else {
            QTreeKey::new(self.depth() + 1, self.x().shl(1), self.y().shl(1))
        }
    }

    /// Returns the key of the right top child node.
    pub fn child_rt(&self) -> Option<QTreeKey> {
        if self.depth() > 28 {
            None
        } else {
            QTreeKey::new(self.depth() + 1, self.x().shl(1) + 1u32, self.y().shl(1))
        }
    }

    /// Returns the key of the left bottom child node.
    pub fn child_lb(&self) -> Option<QTreeKey> {
        if self.depth() > 28 {
            None
        } else {
            QTreeKey::new(self.depth() + 1, self.x().shl(1), self.y().shl(1) + 1u32)
        }
    }

    /// Returns the key of the right bottom child node.
    pub fn child_rb(&self) -> Option<QTreeKey> {
        if self.depth() > 28 {
            None
        } else {
            QTreeKey::new(
                self.depth() + 1,
                self.x().shl(1) + 1u32,
                self.y().shl(1) + 1u32,
            )
        }
    }

    pub fn new(depth: u8, x: u32, y: u32) -> Option<Self> {
        if depth > 28 {
            return None;
        }
        let mask = 0xfff_ffffu64.shr(28 - depth);
        let mut key = 0u64;
        key |= (depth as u64) << 56;
        key |= ((x as u64) & mask) << 28;
        key |= (y as u64) & mask;
        Some(Self(key))
    }

    pub fn root() -> Self {
        QTreeKey(0)
    }

    pub fn inner_key(&self) -> u64 {
        self.0
    }
}

impl Display for QTreeKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "key:[depth:{},x:{},y:{}]",
            self.depth(),
            self.x(),
            self.y()
        )
    }
}

pub trait ReadonlyQTree<T> {
    fn get(&self, key: QTreeKey) -> Option<&QTreeNode<T>>;
    fn get_mut(&mut self, key: QTreeKey) -> Option<&mut QTreeNode<T>>;
}

pub struct QTree<T> {
    data: FxHashMap<u64, QTreeNode<T>>,
}

pub struct QTreeNode<T> {
    data: Option<T>,
}

impl<T> QTreeNode<T> {
    pub fn new(data: T) -> Self {
        Self { data: Some(data) }
    }
}

impl<T> QTree<T> {
    pub fn new() -> Self {
        Self {
            data: FxHashMap::default(),
        }
    }

    pub fn get(&self, key: QTreeKey) -> Option<&QTreeNode<T>> {
        self.data.get(&key.0)
    }

    pub fn get_mut(&mut self, key: QTreeKey) -> Option<&mut QTreeNode<T>> {
        self.data.get_mut(&key.0)
    }

    /// Inserts a node into the tree. The parent node will be created if it does not exist.
    pub fn insert(&mut self, key: QTreeKey, data: T) -> Option<QTreeNode<T>> {
        let mut p = key.parent();
        while let Some(parent) = p {
            if self.data.get_mut(&parent.0).is_none() {
                self.data.insert(parent.0, QTreeNode { data: None });
            } else {
                break;
            }
            p = parent.parent()
        }
        self.data.insert(key.0, QTreeNode { data: Some(data) })
    }

    pub fn size(&self) -> usize {
        self.data.len()
    }

    /// Removes a node from the tree. The node and its children will be removed.
    pub fn remove(&mut self, key: QTreeKey) {
        let mut stack = vec![key];
        self.data.remove(&key.0);
        while let Some(key) = stack.pop() {
            key.child_lt().map(|f| {
                if self.data.remove(&f.0).is_some() {
                    stack.push(f)
                }
            });
            key.child_rt().map(|f| {
                if self.data.remove(&f.0).is_some() {
                    stack.push(f)
                }
            });
            key.child_lb().map(|f| {
                if self.data.remove(&f.0).is_some() {
                    stack.push(f)
                }
            });
            key.child_rb().map(|f| {
                if self.data.remove(&f.0).is_some() {
                    stack.push(f)
                }
            });
        }
    }

    /// Walks the tree from the given key to the target depth from top to bottom.
    pub fn walk(
        &self,
        key: QTreeKey,
        target_depth: u8,
    ) -> impl Iterator<Item = (QTreeKey, &QTreeNode<T>)> {
        let mut stack = VecDeque::new();
        stack.push_back(key);
        std::iter::from_fn(move || {
            if let Some(key) = stack.pop_front() {
                let node = self.get(key);
                if let Some(node) = node {
                    if key.depth() < target_depth {
                        key.child_lt().map(|f| {
                            if self.data.contains_key(&f.0) {
                                stack.push_back(f)
                            }
                        });
                        key.child_rt().map(|f| {
                            if self.data.contains_key(&f.0) {
                                stack.push_back(f)
                            }
                        });
                        key.child_lb().map(|f| {
                            if self.data.contains_key(&f.0) {
                                stack.push_back(f)
                            }
                        });
                        key.child_rb().map(|f| {
                            if self.data.contains_key(&f.0) {
                                stack.push_back(f)
                            }
                        });
                    }
                    Some((key, node))
                } else {
                    None
                }
            } else {
                None
            }
        })
    }
}

impl<T> ReadonlyQTree<T> for QTree<T> {
    fn get(&self, key: QTreeKey) -> Option<&QTreeNode<T>> {
        self.get(key)
    }

    fn get_mut(&mut self, key: QTreeKey) -> Option<&mut QTreeNode<T>> {
        self.get_mut(key)
    }
}

#[test]
fn test_valid() {
    let key = QTreeKey::new(1, 0, 0).unwrap();
    assert!(key.depth() == 1 && key.x() == 0 && key.y() == 0);
    let key1 = key.child_rb().unwrap();
    assert!(key1.depth() == 2 && key1.x() == 1 && key1.y() == 1);
    let key2 = QTreeKey::new(12, 4096, 8191).unwrap();
    assert!(key2.depth() == 12 && key2.x() == 0 && key2.y() == 4095);
    let key3 = key2.child_lb().unwrap();
    assert!(key3.depth() == 13 && key3.x() == 0 && key3.y() == 8191);
    let key4 = key3.left();
    assert!(key4.depth() == 13 && key4.x() == 8191 && key4.y() == 8191);

    let key5 = QTreeKey::new(12, 2333, 2334).unwrap();
    assert!(key5.left() == QTreeKey::new(12, 2332, 2334).unwrap());
    assert!(key5.right() == QTreeKey::new(12, 2334, 2334).unwrap());
    assert!(key5.top() == QTreeKey::new(12, 2333, 2333).unwrap());
    assert!(key5.bottom() == QTreeKey::new(12, 2333, 2335).unwrap());
}

#[test]
fn test_tree() {
    let mut tree = QTree::new();
    tree.insert(QTreeKey::root(), "1");
    tree.insert(QTreeKey::new(1, 0, 0).unwrap(), "2");
    tree.insert(QTreeKey::new(27, 12353, 99910).unwrap(), "3");
    tree.insert(QTreeKey::new(27, 131072, 999999).unwrap(), "3");
    tree.insert(QTreeKey::new(1, 0, 1).unwrap(), "s");
    // tree.remove(QTreeKey::new(4, 0, 0));
    tree.walk(QTreeKey::root(), 0xffu8)
        .for_each(|n| println!("{}\t{}", n.0, n.1.data.unwrap_or("default")));
}
