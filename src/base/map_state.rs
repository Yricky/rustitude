use std::{
    fmt::Display,
    ops::{Add, Sub},
    sync::Arc,
};

use rustc_hash::FxHashMap;

use super::qtree::QTreeKey;

#[derive(Clone, Copy)]
pub struct Location {
    pub x: f64,
    pub y: f64,
}

impl Sub for Location {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Self {
            x: self.x - rhs.x,
            y: self.y - rhs.y,
        }
    }
}

impl Add for Location {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self {
            x: self.x + rhs.x,
            y: self.y + rhs.y,
        }
    }
}

impl Location {
    pub fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }

    pub fn from_qtree_key(key: QTreeKey) -> Self {
        let x = (key.x() as f64) / 2.0_f64.powf(key.depth() as f64);
        let y = (key.y() as f64) / 2.0_f64.powf(key.depth() as f64);
        Self { x, y }
    }

    pub fn as_qtree_key(&self, depth: u8) -> Option<QTreeKey> {
        if depth > 28 {
            return None;
        }
        let x = (self.x * 2.0_f64.powf(depth as f64) - 0.00001) as u32;
        let y = (self.y * 2.0_f64.powf(depth as f64) - 0.00001) as u32;
        QTreeKey::new(depth, x, y)
    }
}

pub fn walk(lt: QTreeKey, rb: QTreeKey) -> impl Iterator<Item = QTreeKey> {
    if lt.depth() != rb.depth() {
        panic!("depth not equal");
    }
    let mut row_head = rb;
    let mut curr = Some(row_head);
    std::iter::from_fn(move || {
        let ret = curr;
        if let Some(c) = curr {
            if c.x() == lt.x() {
                if c.y() == lt.y() {
                    curr = None;
                } else {
                    row_head = row_head.top();
                    curr = Some(row_head);
                }
            } else {
                curr = Some(c.left());
            }
        }
        ret
    })
}

#[test]
fn test_location() {
    println!("{}", Location::new(1.0, 1.0).as_qtree_key(28).unwrap());
    println!("{}", Location::new(0.0, 0.0).as_qtree_key(28).unwrap());
    println!("{}", Location::new(1.0, 1.0).as_qtree_key(1).unwrap());
    println!("{}", Location::new(0.0, 0.0).as_qtree_key(1).unwrap());
    println!("{}", Location::new(0.75, 0.75).as_qtree_key(1).unwrap());
    println!("walk");
    walk(
        QTreeKey::new(27, 10, 10).unwrap(),
        QTreeKey::new(27, 14, 11).unwrap(),
    )
    .for_each(|k| {
        println!("{}", k);
    });
}

impl Display for Location {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Location({}, {})", self.x, self.y)
    }
}

pub enum MapItem {
    Point {
        location: Location,
        data: MapItemData,
    },
    Line {
        locations: Vec<Location>,
        data: MapItemData,
    },
    Polygon {
        locations: Vec<Location>,
        data: MapItemData,
    },
}

pub struct MapItemData {
    name: String,
    props: FxHashMap<String, String>,
}

pub struct MapTileData {
    items: Vec<Arc<MapItem>>,
}

impl Display for MapTileData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "tileDataSize:{}", self.items.len())
    }
}
