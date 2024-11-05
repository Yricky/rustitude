use std::{
    ops::Shl,
    sync::{Arc, RwLock},
};

use egui::{load::SizedTexture, pos2, vec2, Color32, Context, Painter, Rect, Vec2};
use rustitude_base::{map_view_state::MapViewState, qtree::QTreeKey};

pub mod egui_map;

pub const TILE_SIZE_VEC2: Vec2 = vec2(256.0, 256.0);

pub trait EguiDrawable: Send + Sync {
    fn draw(&self, painter: &Painter, rect: Rect);
    fn clip(&self, rect: Rect) -> CommonEguiDrawable;
}

pub type CommonEguiDrawable = Arc<dyn EguiDrawable>;

pub trait EguiMapImgRes {
    fn get(&self, key: QTreeKey) -> Option<CommonEguiDrawable>;

    fn get_or_update(
        &self,
        key: QTreeKey,
        mvs: Arc<RwLock<MapViewState>>,
        ctx: &Context,
    ) -> Option<CommonEguiDrawable>;
}

impl EguiDrawable for SizedTexture {
    fn draw(&self, painter: &Painter, rect: Rect) {
        painter.image(
            self.id,
            rect,
            Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)),
            Color32::WHITE,
        );
    }

    fn clip(&self, rect: Rect) -> CommonEguiDrawable {
        let d = (self.clone(), rect);
        Arc::new(d)
    }
}

impl EguiDrawable for (SizedTexture, Rect) {
    fn draw(&self, painter: &Painter, rect: Rect) {
        painter.image(self.0.id, rect, self.1, Color32::WHITE);
    }

    fn clip(&self, rect: Rect) -> CommonEguiDrawable {
        let self_size = self.1.size();
        let rect_size = rect.size();
        Arc::new((
            self.0,
            Rect::from_min_size(
                pos2(
                    self.1.min.x + self_size.x * rect.min.x,
                    self.1.min.y + self_size.y * rect.min.y,
                ),
                vec2(self_size.x * rect_size.x, self_size.y * rect_size.y),
            ),
        ))
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
