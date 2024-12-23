pub mod egui_map;
pub mod tile_drawable;

use std::{
    ops::Shl,
    sync::{Arc, RwLock},
};

use egui::{pos2, vec2, Context, Rect};
use rustitude_base::{map_view_state::MapViewState, qtree::QTreeKey};
use tile_drawable::CommonEguiTileDrawable;

pub trait EguiMapTileRes {
    fn get_memory_cache(&self, key: QTreeKey) -> Option<CommonEguiTileDrawable>;

    fn get_or_update(
        &self,
        key: QTreeKey,
        mvs: Arc<RwLock<MapViewState>>,
        ctx: &Context,
    ) -> Option<CommonEguiTileDrawable>;
}

pub struct DebugPrintKeyTileRes;
impl EguiMapTileRes for DebugPrintKeyTileRes {
    fn get_memory_cache(&self, key: QTreeKey) -> Option<CommonEguiTileDrawable> {
        Some(Arc::new(key))
    }

    fn get_or_update(
        &self,
        key: QTreeKey,
        _mvs: Arc<RwLock<MapViewState>>,
        _ctx: &Context,
    ) -> Option<CommonEguiTileDrawable> {
        Some(Arc::new(key))
    }
}

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
