use std::ops::Shl;

use ::egui::{pos2, vec2, Rect};
use rustitude_base::qtree::QTreeKey;

pub mod egui;
pub mod priv_fn;

pub fn clip_from_top_key(top_key: QTreeKey, key: QTreeKey) -> Rect {
    let z = key.depth() - top_key.depth();
    let top_key_lt = (top_key.x().shl(z), top_key.y().shl(z));
    let block_count = 1_u32.shl(z);
    let rel_x = (key.x() - top_key_lt.0) as f32 / block_count as f32;
    let rel_y = (key.y() - top_key_lt.1) as f32 / block_count as f32;
    let rel_size = 1.0 / block_count as f32;
    Rect::from_min_size(pos2(rel_x, rel_y), vec2(rel_size, rel_size))
}

#[test]
fn test_clip_key() {
    let key0 = QTreeKey::root();
    let key1 = key0.child_rb().unwrap().child_lb().unwrap();
    println!("{}", clip_from_top_key(key0, key1));
}
